//! Library, Metaxploit, debug, network-session, and administrator routes.

mod admin;
mod admin_network;
mod debug_library;
mod meta_lib;
mod metaxploit;
mod net_session;
mod protocol;

use greyhack_world_core::WorldError;
use greyhack_world_router::RouteError;

use crate::LibraryContext;

/// Failure that prevents a library route from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result returned by a library route.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;

/// Registers every libraries route for concrete world `W`.
pub fn register<W: LibraryContext + 'static>(routes: &greyhack_world_router::RouteTable) {
    admin::register::<W>(routes);
    admin_network::register::<W>(routes);
    debug_library::register::<W>(routes);
    meta_lib::register::<W>(routes);
    metaxploit::register::<W>(routes);
    net_session::register::<W>(routes);
}
