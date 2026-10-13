//! `duplex-core` WebSocket provider with one transport per client and one binary message per batch.

#![warn(missing_docs)]
mod driver;
mod options;
mod server;

pub use driver::WebSocketDriver;
pub use options::WebSocketOptions;
pub use server::WebSocketServer;
