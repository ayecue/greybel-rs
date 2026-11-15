//! Connection errors grouped by the Grey Hack procedure that rejects the request. The runtime
//! formats each error using its intrinsic.

mod connection;
mod login;
mod service;

pub use connection::ConnectionFailure;
pub use login::LoginFailure;
pub use service::ServiceFailure;
