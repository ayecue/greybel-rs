//! Guild administration routes and their world-protocol conversions.

mod admin;
mod admin_network;
mod protocol;

use greyhack_world_core::WorldError;
use greyhack_world_router::RouteError;

use crate::GuildContext;

/// Failure that prevents a guild route from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result returned by a guild route.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;

/// Registers every guild route for concrete world `W`.
pub fn register<W: GuildContext + 'static>(routes: &greyhack_world_router::RouteTable) {
    admin::register::<W>(routes);
    admin_network::register::<W>(routes);
}
