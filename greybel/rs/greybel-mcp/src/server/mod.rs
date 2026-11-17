//! MCP server and per-tool modules.

mod api;
mod build;
mod check;
mod code;
mod import;
mod mode;
mod run;
mod source;
mod transform;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ServerHandler, tool_handler};

const INSTRUCTIONS: &str = "Tools for GreyScript, the language of the game Grey Hack, and MiniScript, which it extends: \
look up the API (api_search, api_reference), check code for errors (check), navigate a project's code by symbol name \
(document_symbols, find_symbol, find_references, describe, call_hierarchy), rename symbols (rename), transform code \
(transform), build projects with what they import (build) and run programs in a simulated Grey Hack world (run). In \
the simulated world, `get_shell(\"root\", \"test\")` is the local root shell.";

/// Configuration for Grey Hack message-hook access.
#[derive(Debug, Clone, Copy)]
pub struct ServerOptions {
    /// Enables game execution and file-import tools through the message hook.
    pub allow_game: bool,
    /// Game message-hook port.
    pub port: u16,
}

/// MCP tool server for one client configuration.
#[derive(Clone)]
pub struct Server {
    options: ServerOptions,
    tool_router: ToolRouter<Self>,
}

impl Server {
    /// Creates a server with all tools; game tools are included only when allowed by `options`.
    #[must_use]
    pub fn new(options: ServerOptions) -> Self {
        let mut tool_router = Self::api_router()
            + Self::check_router()
            + Self::code_router()
            + Self::transform_router()
            + Self::build_router()
            + Self::run_router();

        if options.allow_game {
            tool_router += Self::import_router();
        }

        Self {
            options,
            tool_router,
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("greybel", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}
