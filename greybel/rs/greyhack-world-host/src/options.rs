//! Command-line options of a world server.

use std::path::PathBuf;
use std::time::Duration;

/// Port, seed, storage and lifetime of a world server.
#[derive(clap::Args, Clone, Debug)]
pub struct ServeOptions {
    /// Listen on this TCP port; 0 selects any free port.
    #[arg(long, default_value_t = 0)]
    pub port: u16,
    /// Seed used to generate a new world.
    #[arg(long, default_value = "test")]
    pub seed: String,
    /// Load and save this database file instead of the seed's world in the worlds folder.
    #[arg(long, conflicts_with = "memory")]
    pub database: Option<PathBuf>,
    /// Folder of saved worlds, one per seed, indexed by `worlds.json` [default: `greybel/worlds` in the user's data
    /// folder].
    #[arg(long, value_name = "FOLDER", env = "GREYBEL_WORLDS")]
    pub worlds: Option<PathBuf>,
    /// Store the world in memory only, so its changes are lost when the server stops.
    #[arg(long)]
    pub memory: bool,
    /// Stop after this many seconds without connections; 0 never stops. Without it, the server runs until its
    /// standard input closes.
    #[arg(long, value_name = "SECONDS")]
    pub idle_timeout: Option<u64>,
}

/// When a server stops.
pub(crate) enum Lifetime {
    /// When its standard input closes.
    Input,
    /// After this time without connections.
    Idle(Duration),
    Endless,
}

impl ServeOptions {
    pub(crate) fn lifetime(&self) -> Lifetime {
        match self.idle_timeout {
            None => Lifetime::Input,
            Some(0) => Lifetime::Endless,
            Some(seconds) => Lifetime::Idle(Duration::from_secs(seconds)),
        }
    }
}
