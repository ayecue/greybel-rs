//! World sources for `greybel-runtime` hosts without an editor, including the CLI and MCP server: generate a world
//! from a seed in process or connect to a world server. Editors share one world server instead.

mod local;
#[cfg(not(target_family = "wasm"))]
mod source;

pub use local::{LocalWorld, LocalWorldFactory};
#[cfg(not(target_family = "wasm"))]
pub use source::WorldSource;
