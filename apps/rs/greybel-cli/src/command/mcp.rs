use clap::Args;
use greybel_mcp::ServerOptions;

use super::block_on;
use super::options::{MessageHook, WorldOptions};
use crate::report::Failure;

#[derive(Debug, Args)]
pub struct McpCommand {
    /// Enables MCP program execution and file imports in Grey Hack's single-player game through its message hook.
    #[arg(long = "allow-game")]
    allow_game: bool,
    #[command(flatten)]
    message_hook: MessageHook,
    #[command(flatten)]
    world: WorldOptions,
}

impl McpCommand {
    /// Serves Greybel tools over MCP on stdin/stdout until the client disconnects.
    pub fn run(self) -> Result<(), Failure> {
        let options = ServerOptions {
            allow_game: self.allow_game,
            port: self.message_hook.port,
            world: self.world.source()?,
        };

        block_on(greybel_mcp::serve(options)).map_err(|message| Failure::new("MCP error", message))
    }
}
