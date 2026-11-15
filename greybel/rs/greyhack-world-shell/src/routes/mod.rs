//! Shell and administrator routes and their world-protocol conversions.

mod admin;
mod protocol;
pub mod shell;

use greyhack_world_core::WorldError;
use greyhack_world_programs::RouteError;

/// Failure that prevents a shell route from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result returned by a shell route.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;

/// Registers every shell route for concrete world `W`.
pub fn register<
    W: greyhack_world_router::WorldContext<
            Computer = greyhack_world_core::ComputerId,
            Error = greyhack_world_core::WorldError,
        > + 'static,
>(
    routes: &greyhack_world_router::RouteTable,
) {
    admin::register::<W>(routes);
    shell::register::<W>(routes);
}
