//! NPC routes and their world-protocol conversions.

mod admin;
mod admin_computer;
mod admin_network;
mod meta_mail;
mod protocol;
mod smtp;

use greyhack_world_core::WorldError;
use greyhack_world_programs::RouteError;

/// Failure that prevents an NPC route from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result returned by an NPC route.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;

/// Registers every NPC route for concrete world `W`.
pub fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &greyhack_world_router::RouteTable,
) {
    admin::register::<W>(routes);
    admin_computer::register::<W>(routes);
    admin_network::register::<W>(routes);
    meta_mail::register::<W>(routes);
    smtp::register::<W>(routes);
}
