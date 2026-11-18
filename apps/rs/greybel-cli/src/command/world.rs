//! Serves and resets the simulated world shared by the runtimes of this machine.

use clap::{Args, Subcommand};
use greybel_runtime::{DEFAULT_PORT, SharedWorld};
use greyhack_world_host::{ServeOptions, WorldHost};

use super::block_on;
use crate::report::Failure;

#[derive(Debug, Args)]
pub struct WorldCommand {
    #[command(subcommand)]
    action: WorldAction,
}

#[derive(Debug, Subcommand)]
enum WorldAction {
    /// Serves a simulated world on a loopback port and prints the port on the first output line.
    Serve(ServeOptions),
    /// Regenerates the shared server's world, interrupting programs running in it.
    Reset {
        /// Port of the shared world server.
        #[arg(long, value_name = "PORT", default_value_t = DEFAULT_PORT)]
        world_port: u16,
    },
}

impl WorldCommand {
    pub fn run(self) -> Result<(), Failure> {
        match self.action {
            WorldAction::Serve(options) => block_on(WorldHost::serve(options))
                .map_err(|error| Failure::new("World error", error.to_string())),
            WorldAction::Reset { world_port } => {
                let seed = block_on(SharedWorld::reset(world_port))
                    .map_err(|failure| Failure::new("World error", failure.to_string()))?;

                println!("The world of seed \"{seed}\" on port {world_port} was generated again.");

                Ok(())
            }
        }
    }
}
