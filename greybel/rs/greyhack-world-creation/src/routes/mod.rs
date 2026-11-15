//! Network-creation administrator routes and protocol conversions.

mod admin;
mod admin_network;
mod protocol;

use greyhack_world_core::WorldError;
use greyhack_world_router::RouteError;

/// Failure that prevents a creation route from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result returned by a creation route.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;

/// Registers every creation route for concrete world `W`.
pub fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &greyhack_world_router::RouteTable,
) {
    admin::register::<W>(routes);
    admin_network::register::<W>(routes);
}
