//! Connection errors grouped by the Grey Hack procedure that rejects the request. The runtime
//! formats each error using its intrinsic.

mod connection;
mod login;
mod net_use;
mod rshell;
mod service;
mod smtp;

pub use connection::ConnectionFailure;
pub use greyhack_behaviour::LanFailure;
pub use login::LoginFailure;
pub use net_use::NetUseFailure;
pub use rshell::RshellFailure;
pub use service::ServiceFailure;
pub use smtp::SmtpFailure;
