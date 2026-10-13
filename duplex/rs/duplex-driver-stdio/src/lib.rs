//! Tokio `duplex-core` drivers for length-prefixed byte streams ([`StreamDriver`]) and spawned-channel ports
//! ([`PortDriver`]).

#![warn(missing_docs)]
mod port;
mod stream;

pub use port::PortDriver;
pub use stream::StreamDriver;
