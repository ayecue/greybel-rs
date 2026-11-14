//! Domain routes and protocol conversions.

pub mod admin;
pub mod admin_computer;
pub mod admin_network;

mod cycles;

pub use cycles::CyclesExt;

pub(crate) fn register(routes: &greyhack_world_router::RouteTable) {
    admin::register(routes);
    admin_computer::register(routes);
    admin_network::register(routes);
}

pub use greyhack_world_core::routes::conversions;
use greyhack_world_router::RouteError;
pub use greyhack_world_router::TerminalInfo;

use crate::WorldError;

/// Failure that prevents a route from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result returned by a domain route.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;
