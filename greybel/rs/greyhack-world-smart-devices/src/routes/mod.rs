//! Smart-device routes and their world-protocol conversions.

mod admin;
mod admin_computer;
mod protocol;
mod smart_appliance;
mod traffic;

use greyhack_world_core::WorldError;
use greyhack_world_router::RouteError;

/// Failure that prevents a smart-device route from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result returned by a smart-device route.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;

/// Registers every smart-device route for concrete world `W`.
pub fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &greyhack_world_router::RouteTable,
) {
    admin::register::<W>(routes);
    admin_computer::register::<W>(routes);
    smart_appliance::register::<W>(routes);
    traffic::register::<W>(routes);
}
