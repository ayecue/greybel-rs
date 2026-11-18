//! Options for selecting the world used by simulated programs.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use clap::Args;
use greybel_runtime::{DEFAULT_IDLE_TIMEOUT, DEFAULT_PORT, ServerCommand, SharedWorld};
use greybel_runtime_world::WorldSource;

use crate::report::Failure;

/// World server options of commands that run programs in the simulated world.
#[derive(Debug, Clone, Args)]
pub struct WorldOptions {
    /// Address of a world server to run programs in, instead of the world server shared on this machine.
    #[arg(long, value_name = "ADDRESS")]
    pub world: Option<String>,
    /// Run each program in a world of its own instead of the world server shared on this machine. `--world` takes
    /// precedence.
    #[arg(long, env = "GREYBEL_OWN_WORLD")]
    pub own_world: bool,
    /// Port of the world server shared on this machine. When none runs there, the command starts one, which other
    /// commands and editors use too.
    #[arg(long, value_name = "PORT", default_value_t = DEFAULT_PORT)]
    pub world_port: u16,
    /// Seconds without connections before a shared server started by this command stops; 0 disables the timeout.
    #[arg(long, value_name = "SECONDS", default_value_t = DEFAULT_IDLE_TIMEOUT.as_secs())]
    pub world_idle_timeout: u64,
    /// Folder where a shared server started by this command saves each seed's world [default: `greybel/worlds` in
    /// the user's data folder].
    #[arg(long, value_name = "FOLDER", env = "GREYBEL_WORLDS")]
    pub worlds: Option<PathBuf>,
}

impl WorldOptions {
    /// Returns the selected world source; shared servers are started with `greybel world serve`.
    pub fn source(&self) -> Result<WorldSource, Failure> {
        if let Some(address) = &self.world {
            return Ok(WorldSource::Address(address.clone()));
        }

        if self.own_world {
            return Ok(WorldSource::Own);
        }

        let program = std::env::current_exe().map_err(|error| {
            Failure::new(
                "World error",
                format!("greybel can't find its program file: {error}"),
            )
        })?;

        Ok(WorldSource::Machine(SharedWorld::new(
            self.world_port,
            Duration::from_secs(self.world_idle_timeout),
            ServerCommand::new(program, self.serve_arguments()),
        )))
    }

    /// Arguments of `greybel` that start a world server, before its port, seed and idle timeout.
    fn serve_arguments(&self) -> Vec<OsString> {
        let mut arguments = vec!["world".into(), "serve".into()];

        if let Some(worlds) = &self.worlds {
            arguments.extend(["--worlds".into(), worlds.clone().into_os_string()]);
        }

        arguments
    }
}
