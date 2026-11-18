//! The game server's process, and programs run against it.

pub mod store;

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use duplex_core::{Channel, ChannelOptions};
use duplex_driver_stdio::StreamDriver;
use greybel_runtime::{ServerCommand, SharedWorld};
use greybel_test_support::interpreter::{inline, snapshot_options};
use greyhack_world::PlayerOptions;
use greyscript_intrinsics::{GreyHackInterpreter, ServerConnection};
use tokio::net::TcpStream;

/// Game-server child process stopped when dropped.
pub struct GameServerProcess {
    child: Child,
    port: u16,
}

impl GameServerProcess {
    /// Starts a server with a memory-only world.
    pub fn start() -> Self {
        Self::start_with(&["--memory"], Stdio::inherit())
    }

    /// Starts a server with `arguments`, writing its error output to `stderr`.
    pub fn start_with(arguments: &[&str], stderr: Stdio) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_greyhack-world-server"))
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .expect("the game server starts");
        let mut line = String::new();

        BufReader::new(child.stdout.as_mut().expect("its output"))
            .read_line(&mut line)
            .expect("it prints its port");

        Self {
            port: line.trim().parse().expect("a port"),
            child,
        }
    }

    /// Closes the server's input, which stops and saves it, waits for it to exit, and returns its error output when
    /// it was captured.
    pub fn stop(mut self) -> String {
        drop(self.child.stdin.take());

        let mut error = String::new();

        if let Some(stderr) = self.child.stderr.as_mut() {
            std::io::Read::read_to_string(stderr, &mut error).expect("the error output is read");
        }

        let _ = self.child.wait();

        error
    }

    pub async fn connect(&self) -> ServerConnection {
        let stream = TcpStream::connect(("127.0.0.1", self.port))
            .await
            .expect("the game server accepts");

        ServerConnection::new(Channel::new(
            &StreamDriver::tcp(stream).expect("the connection is set up"),
            ChannelOptions::default(),
        ))
    }
}

impl Drop for GameServerProcess {
    fn drop(&mut self) {
        drop(self.child.stdin.take());

        let _ = self.child.wait();
    }
}

/// Idle timeout of the shared world servers the tests start.
pub const IDLE: Duration = Duration::from_secs(2);

/// Unused port of the loopback address.
pub fn unused_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .expect("a free port is found")
        .port()
}

/// Empty worlds folder of a test named `name`.
pub fn worlds_folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!(
        "greyhack-world-server-test-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&folder);

    std::fs::create_dir_all(&folder).expect("the worlds folder is created");

    folder
}

/// Shared world server on an unused port, saving worlds in `worlds`, started from this package's program and stopped
/// after `idle_timeout`.
pub fn shared_world(idle_timeout: Duration, worlds: &Path) -> SharedWorld {
    SharedWorld::new(
        unused_port(),
        idle_timeout,
        ServerCommand::new(
            env!("CARGO_BIN_EXE_greyhack-world-server"),
            ["--worlds".into(), worlds.as_os_str().to_owned()],
        ),
    )
}

/// Runs `code` as the default player through `server` and returns its printed output.
pub async fn run(server: &ServerConnection, code: &str) -> String {
    let (lines, options) = snapshot_options();
    let mut program = GreyHackInterpreter::start(
        server,
        &PlayerOptions::default().user,
        options,
        inline(code),
    )
    .await
    .expect("the program starts");

    program.run().await.expect("the program runs to its end");

    lines.text()
}
