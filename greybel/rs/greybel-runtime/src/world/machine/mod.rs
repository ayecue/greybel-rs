//! Finds the world server that the runtimes of this machine share on one loopback port, and starts it when none
//! runs there.

mod handshake;
mod protocol;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use duplex_core::{Channel, ChannelOptions, ErrorCode};
use duplex_driver_stdio::StreamDriver;
use greyscript_intrinsics::ServerConnection;
use greyscript_intrinsics::protocol::Program;
use protocol::{Admin, HostInfo, Reset, ResetFailure};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;

pub const DEFAULT_PORT: u16 = 41740;
/// Idle time without connections before a shared world server stops.
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(600);
/// Time a world server has to respond, including world generation on startup.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(60);

/// Program and leading arguments that start a world server; `--port`, `--seed` and `--idle-timeout` follow them.
#[derive(Clone, Debug)]
pub struct ServerCommand {
    program: PathBuf,
    arguments: Vec<OsString>,
}

impl ServerCommand {
    #[must_use]
    pub fn new(
        program: impl Into<PathBuf>,
        arguments: impl IntoIterator<Item = impl Into<OsString>>,
    ) -> Self {
        Self {
            program: program.into(),
            arguments: arguments.into_iter().map(Into::into).collect(),
        }
    }
}

/// Reason a shared world server can't be used or reset.
#[derive(Debug, thiserror::Error)]
pub enum SharedWorldFailure {
    #[error("the world server on port {port} runs seed \"{seed}\"")]
    Seed { port: u16, seed: String },
    #[error("the world server on port {port} speaks another protocol version: {}", reason.message())]
    Incompatible { port: u16, reason: ErrorCode },
    #[error("port {port} is used by a program other than a world server")]
    Foreign { port: u16 },
    #[error("the world server couldn't start: {source}")]
    Spawn {
        #[source]
        source: std::io::Error,
    },
    #[error("the world server stopped while starting; see {}", log.display())]
    Exit { log: PathBuf },
    #[error("no world server runs on port {port}")]
    Absent { port: u16 },
    #[error(
        "the world server on port {port} stores its world in a database, which a reset would overwrite"
    )]
    Persistent { port: u16 },
}

/// World server shared on a loopback port. Connecting starts it with the connecting seed when none runs there.
#[derive(Clone, Debug)]
pub struct SharedWorld {
    port: u16,
    idle_timeout: Duration,
    command: ServerCommand,
}

impl SharedWorld {
    /// Server on `port`, started by `command` with `idle_timeout`; zero never stops a started server.
    #[must_use]
    pub fn new(port: u16, idle_timeout: Duration, command: ServerCommand) -> Self {
        Self {
            port,
            idle_timeout,
            command,
        }
    }

    #[must_use]
    pub fn address(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    /// Connects to the server's world, starting the server with `seed` when none runs on the port.
    ///
    /// # Errors
    ///
    /// Returns why the server can't be used: it runs another seed or protocol version, the port is used otherwise,
    /// or the server didn't start.
    pub async fn connect(&self, seed: &str) -> Result<ServerConnection, SharedWorldFailure> {
        if let Some(world) = self.probe(seed).await? {
            return Ok(world);
        }

        let log = self.start(seed).await?;

        self.probe(seed)
            .await?
            .ok_or(SharedWorldFailure::Exit { log })
    }

    /// Regenerates the world on `port`, interrupting its programs, and returns the server's seed.
    ///
    /// # Errors
    ///
    /// Returns why the server can't be reset: none runs on the port, it isn't a usable world server, or its world
    /// is stored in a database.
    pub async fn reset(port: u16) -> Result<String, SharedWorldFailure> {
        let (admin, _) = Self::open(port)
            .await?
            .ok_or(SharedWorldFailure::Absent { port })?;
        let reset = admin
            .send(&Reset {})
            .map_err(|_| SharedWorldFailure::Foreign { port })?;

        match reset.await {
            Ok(info) => Ok(info.seed),
            Err(duplex_core::Error::Failure(ResetFailure::Persistent)) => {
                Err(SharedWorldFailure::Persistent { port })
            }
            Err(_) => Err(SharedWorldFailure::Foreign { port }),
        }
    }

    /// Connects to the server's world when it runs `seed`; `None` when no server runs on the port.
    async fn probe(&self, seed: &str) -> Result<Option<ServerConnection>, SharedWorldFailure> {
        let port = self.port;
        let Some((admin, info)) = Self::open(port).await? else {
            return Ok(None);
        };

        if info.seed != seed {
            return Err(SharedWorldFailure::Seed {
                port,
                seed: info.seed,
            });
        }

        let world = Channel::<Program, _>::new(admin.transport(), ChannelOptions::default());

        Self::response(port, handshake::ack(&world)).await?;
        admin.close(ErrorCode::ClosedChannel);

        Ok(Some(ServerConnection::new(world)))
    }

    /// Opens the administrator channel of the server on `port`; `None` when nothing listens there.
    async fn open(
        port: u16,
    ) -> Result<Option<(Channel<Admin, StreamDriver>, HostInfo)>, SharedWorldFailure> {
        let Ok(stream) = TcpStream::connect(("127.0.0.1", port)).await else {
            return Ok(None);
        };
        let transport =
            StreamDriver::tcp(stream).map_err(|_| SharedWorldFailure::Foreign { port })?;
        let admin = Channel::<Admin, _>::new(&transport, ChannelOptions::default());
        let info = Self::response(port, handshake::ack(&admin))
            .await?
            .ok_or(SharedWorldFailure::Foreign { port })?;

        Ok(Some((admin, info)))
    }

    /// Waits for `handshake` within [`RESPONSE_TIMEOUT`] and classifies its failure.
    async fn response<T>(
        port: u16,
        handshake: impl Future<Output = Result<T, ErrorCode>>,
    ) -> Result<T, SharedWorldFailure> {
        match timeout(RESPONSE_TIMEOUT, handshake).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(reason @ (ErrorCode::VersionMismatch | ErrorCode::UnsupportedVersion))) => {
                Err(SharedWorldFailure::Incompatible { port, reason })
            }
            Ok(Err(_)) | Err(_) => Err(SharedWorldFailure::Foreign { port }),
        }
    }

    /// Starts a server for `seed` in its own process group, so it outlives this process, and waits until it listens
    /// or stops. A server that loses the port to one started at the same time stops. Returns the server's log file.
    async fn start(&self, seed: &str) -> Result<PathBuf, SharedWorldFailure> {
        let log = std::env::temp_dir().join(format!("greyhack-world-{}.log", self.port));
        let stderr = std::fs::File::create(&log).map_or_else(|_| Stdio::null(), Stdio::from);
        let mut command = tokio::process::Command::new(&self.command.program);

        command
            .args(&self.command.arguments)
            .arg("--port")
            .arg(self.port.to_string())
            .arg("--seed")
            .arg(seed)
            .arg("--idle-timeout")
            .arg(self.idle_timeout.as_secs().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(stderr);
        #[cfg(unix)]
        command.process_group(0);
        #[cfg(windows)]
        {
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;

            command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
        }

        let mut server = command
            .spawn()
            .map_err(|source| SharedWorldFailure::Spawn { source })?;
        let mut line = String::new();

        if let Some(stdout) = server.stdout.take() {
            let _ = timeout(
                RESPONSE_TIMEOUT,
                BufReader::new(stdout).read_line(&mut line),
            )
            .await;
        }

        Ok(log)
    }
}
