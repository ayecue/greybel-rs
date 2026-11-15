#![deny(clippy::await_holding_refcell_ref)]

//! Executes Grey Hack script operations sequentially against a shared world, preventing programs from observing partial operations.
//! Programs still see each other's completed changes.

mod connection;
mod error;
mod server;

pub use error::{OperationError, Result};
pub use server::{TickOptions, WorldServer};
