//! World server that runtimes reach over loopback TCP. Each connection carries the `greyhack-world` protocol for
//! programs and the `greyhack-world-host` protocol, which describes and resets the server.

mod options;
mod protocol;
mod server;
mod store;

pub use options::ServeOptions;
pub use server::{ServeError, WorldHost};
