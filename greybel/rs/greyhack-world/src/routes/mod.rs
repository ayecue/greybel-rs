//! Domain routes and protocol conversions.

pub mod admin;

mod cycles;

pub use cycles::CyclesExt;

pub(crate) fn register(routes: &greyhack_world_router::RouteTable) {
    admin::register(routes);
}

use greyhack_world_core::WorldError;
use greyhack_world_programs::RouteError;

/// Failure that prevents a route from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result returned by a domain route.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;
