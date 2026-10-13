//! Tokio utilities shared by `duplex-core` drivers. `scheduler::TokioScheduler` runs transport tasks on the current
//! [`tokio::task::LocalSet`]; [`HubManager`] runs one hub per worker thread.

#![warn(missing_docs)]
mod acceptor;
mod hub_manager;
mod random;
mod read_gate;

pub use acceptor::Acceptor;
pub use hub_manager::HubManager;
pub use random::random_bytes;
pub use read_gate::ReadGate;
