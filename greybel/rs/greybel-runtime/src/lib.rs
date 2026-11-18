//! Executes and debugs GreyScript programs. [`Runtime`] runs one launch at a time with [`Host`] resources and
//! [`Controller`] commands. [`EditorHost`] connects editors through duplex runtime, io, and documents protocols.

mod breakpoints;
mod editor;
mod program;
pub mod protocol;
mod runtime;
mod uri;
mod world;

pub use editor::EditorHost;
pub use program::PLAYER_USER;
pub use runtime::{Controller, Host, Io, Runtime, Yielder};
#[cfg(not(target_family = "wasm"))]
pub use world::machine::{
    DEFAULT_IDLE_TIMEOUT, DEFAULT_PORT, ServerCommand, SharedWorld, SharedWorldFailure,
};
#[cfg(not(target_family = "wasm"))]
pub use world::tcp::connect_over_tcp;
pub use world::{LaunchWorld, WorldFactory, WorldFuture};
