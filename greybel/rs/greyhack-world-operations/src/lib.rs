#![deny(clippy::await_holding_refcell_ref)]

//! Executes Grey Hack script operations sequentially against a shared world, preventing programs from observing partial operations.
//! Programs still see each other's completed changes. [`protocol`] defines requests sent by connected runtimes.

mod connection;
mod error;
pub mod protocol;
mod server;

pub use error::{OperationError, Result};
pub use server::{TickOptions, WorldServer};
