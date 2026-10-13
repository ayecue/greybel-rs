//! Grey Hack GreyScript intrinsics running client-side against the game server.
//! Intentional differences from Grey Hack are documented in `PORTING.md`.

mod error;
mod game;
mod natives;
pub mod protocol;
pub mod regex;
mod runtime;
pub use error::IntrinsicError;
pub use game::{GAME_API, Game, Session};
pub use natives::API;
pub use runtime::{
    ChildRunner, GreyHackError, GreyHackInterpreter, ProgramBinary, ServerConnection,
};
