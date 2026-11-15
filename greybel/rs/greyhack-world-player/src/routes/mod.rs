//! Player, CTF, and administrator routes and their world-protocol conversions.

mod admin;
mod ctf;
mod player;
mod protocol;

use greyhack_world_core::WorldError;
use greyhack_world_router::RouteError;

/// Failure that prevents a player route from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result returned by a player route.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;

/// Registers every player route for concrete world `W`.
pub fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &greyhack_world_router::RouteTable,
) {
    admin::register::<W>(routes);
    ctf::register::<W>(routes);
    player::register::<W>(routes);
}
