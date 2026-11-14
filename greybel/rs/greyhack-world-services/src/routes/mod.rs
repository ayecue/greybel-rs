//! Package-manager, service-control, and service-administration routes.

mod admin_computer;
mod apt_client;
mod protocol;
mod service;

use greyhack_world_core::WorldError;
use greyhack_world_router::RouteError;

use crate::ServicesContext;

/// Failure that prevents a services route from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result returned by a services route.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;

/// Registers every services route for concrete world `W`.
pub fn register<W: ServicesContext + 'static>(routes: &greyhack_world_router::RouteTable) {
    admin_computer::register::<W>(routes);
    apt_client::register::<W>(routes);
    service::register::<W>(routes);
}
