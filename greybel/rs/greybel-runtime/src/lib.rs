//! Executes and debugs GreyScript programs. [`Runtime`] runs one launch at a time with [`Host`] resources and
//! [`Controller`] commands. [`EditorHost`] connects editors through duplex runtime, io, and documents protocols.

mod breakpoints;
mod editor;
mod program;
pub mod protocol;
mod runtime;
mod uri;

pub use editor::EditorHost;
pub use program::mock_world_server;
pub use runtime::{Controller, Host, Io, Runtime, Yielder};
#[cfg(not(target_family = "wasm"))]
pub use runtime::{connect_on_tokio, connect_over_tcp};
