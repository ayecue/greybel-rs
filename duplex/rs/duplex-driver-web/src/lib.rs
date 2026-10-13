//! Browser `duplex-core` driver with `postMessage` transports ([`PostMessage`]) and shared worker channel state
//! ([`WorkerScope`]).

#![warn(missing_docs)]
mod driver;
mod worker;

pub use driver::{MessageTarget, PostMessage};
pub use worker::{EventLoop, WorkerListener, WorkerScope};
