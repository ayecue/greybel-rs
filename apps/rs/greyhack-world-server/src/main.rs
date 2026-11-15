//! Serve a shared world through TCP on 127.0.0.1. Print the selected port on the first output line and exit after
//! stdin closes.

use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
use std::rc::Rc;

use clap::Parser;
use duplex_driver_stdio::StreamDriver;
use greyhack_world::{DatabaseOpen, PlayerOptions, World};
use greyhack_world_core::{Database, WorldError};
use greyhack_world_operations::WorldServer;
use scheduler::{Clock, TokioScheduler};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::task::LocalSet;

/// Command-line options for a world server shared across separate runtime processes.
#[derive(Parser)]
#[command(name = "greyhack-world-server")]
struct Options {
    /// Listen on this TCP port; 0 selects any free port.
    #[arg(long, default_value_t = 0)]
    port: u16,
    /// Seed used to generate a new world.
    #[arg(long, default_value = "test")]
    seed: String,
    /// Load and save this database file; use an in-memory world when omitted.
    #[arg(long)]
    database: Option<PathBuf>,
}

/// World server runtime error.
#[derive(thiserror::Error)]
enum WorldServerError {
    #[error("{0}")]
    Runtime(#[source] io::Error),
    #[error("{0}")]
    Serve(#[from] ServeError),
}

/// Matches the debug representation of the `String` error this binary returned before.
impl fmt::Debug for WorldServerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.to_string(), formatter)
    }
}

/// World serving error.
#[derive(Debug, thiserror::Error)]
enum ServeError {
    #[error("{0}")]
    Open(#[from] OpenError),
    #[error("{source}")]
    Bind {
        _port: u16,
        #[source]
        source: io::Error,
    },
    #[error("{0}")]
    LocalAddress(#[source] io::Error),
    #[error("{0}")]
    Output(#[source] io::Error),
    #[error("{0}")]
    Save(#[source] WorldError),
}

/// World open error.
#[derive(Debug, thiserror::Error)]
enum OpenError {
    #[error("{0}")]
    World(#[from] WorldError),
    #[error("{source}")]
    Database {
        _path: PathBuf,
        #[source]
        source: WorldError,
    },
}

fn main() -> Result<(), WorldServerError> {
    let options = Options::parse();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(WorldServerError::Runtime)?;

    LocalSet::new()
        .block_on(&runtime, serve(options))
        .map_err(WorldServerError::from)
}

async fn serve(options: Options) -> Result<(), ServeError> {
    let server = open(&options).await?;
    let listener = TcpListener::bind(("127.0.0.1", options.port))
        .await
        .map_err(|source| ServeError::Bind {
            _port: options.port,
            source,
        })?;
    let port = listener
        .local_addr()
        .map_err(ServeError::LocalAddress)?
        .port();
    let mut stdout = std::io::stdout();

    writeln!(stdout, "{port}")
        .and_then(|()| stdout.flush())
        .map_err(ServeError::Output)?;

    let accepts = async {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };

            if let Ok(transport) = StreamDriver::tcp(stream) {
                server.attach(&transport);
            }
        }
    };
    let mut input = tokio::io::stdin();
    let end = async {
        let mut buffer = [0; 64];

        while input.read(&mut buffer).await.is_ok_and(|count| count > 0) {}
    };

    tokio::select! {
        () = accepts => {}
        () = end => {}
    }

    server.save().await.map_err(ServeError::Save)
}

/// Load an existing database world, initialize a new database world, or create a memory-only world using the seed.
async fn open(options: &Options) -> Result<WorldServer, OpenError> {
    let world = async || World::from_seed(&options.seed, &PlayerOptions::default()).await;
    let clock = || -> Rc<dyn Clock> { Rc::new(TokioScheduler::new()) };
    let Some(path) = &options.database else {
        return Ok(WorldServer::new(world().await?, clock()).await?);
    };
    let exists = path.exists();
    let database = Database::open(&path.to_string_lossy())
        .await
        .map_err(|error| OpenError::Database {
            _path: path.clone(),
            source: WorldError::from(error),
        })?;

    if exists {
        return Ok(WorldServer::open(database, clock()).await?);
    }

    Ok(WorldServer::with_database(world().await?, database, clock()).await?)
}
