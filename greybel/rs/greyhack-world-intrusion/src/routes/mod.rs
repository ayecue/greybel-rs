//! Intrusion routes and their world-protocol conversions.

mod crypto;
mod metaxploit;
mod net_session;
mod protocol;

use greyhack_world_core::WorldError;
use greyhack_world_programs::RouteError;

/// Failure that prevents an intrusion route from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result returned by an intrusion route.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;

/// Registers every intrusion route for concrete world `W`.
pub fn register<
    W: greyhack_world_router::WorldContext<
            Computer = greyhack_world_core::ComputerId,
            Error = greyhack_world_core::WorldError,
        > + 'static,
>(
    routes: &greyhack_world_router::RouteTable,
) {
    crypto::register::<W>(routes);
    metaxploit::register::<W>(routes);
    net_session::register::<W>(routes);
}
