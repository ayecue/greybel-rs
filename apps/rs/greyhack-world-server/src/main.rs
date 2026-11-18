//! Serve a world through TCP on 127.0.0.1. Print the selected port on the first output line, then serve until stdin
//! closes or, with `--idle-timeout`, until no runtime has been connected for that long.

use std::{fmt, io};

use clap::Parser;
use greyhack_world_host::{ServeError, ServeOptions, WorldHost};
use tokio::task::LocalSet;

/// Command-line options for a world server shared across separate runtime processes.
#[derive(Parser)]
#[command(name = "greyhack-world-server")]
struct Options {
    #[command(flatten)]
    serve: ServeOptions,
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

fn main() -> Result<(), WorldServerError> {
    let options = Options::parse();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(WorldServerError::Runtime)?;

    LocalSet::new()
        .block_on(&runtime, WorldHost::serve(options.serve))
        .map_err(WorldServerError::from)
}
