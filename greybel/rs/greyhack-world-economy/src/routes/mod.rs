//! Economy routes and their world-protocol conversions.

mod admin;
mod blockchain;
mod coin;
mod protocol;
mod sub_wallet;
mod wallet;

use greyhack_world_core::WorldError;
use greyhack_world_programs::RouteError;

/// Failure that prevents an economy route from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result returned by an economy route.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;

/// Registers every economy route for concrete world `W`.
pub fn register<
    W: greyhack_world_router::WorldContext<
            Computer = greyhack_world_core::ComputerId,
            Error = greyhack_world_core::WorldError,
        > + 'static,
>(
    routes: &greyhack_world_router::RouteTable,
) {
    admin::register::<W>(routes);
    blockchain::register::<W>(routes);
    coin::register::<W>(routes);
    sub_wallet::register::<W>(routes);
    wallet::register::<W>(routes);
}
