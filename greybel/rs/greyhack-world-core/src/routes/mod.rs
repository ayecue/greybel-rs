//! RouteTable and protocol conversions owned by the base computer, file-system, and network services.

pub mod conversions;

mod admin;
pub mod computer;
pub mod file;
pub mod port;
pub mod router;

use greyhack_world_router::RouteError;
use greyhack_world_router::protocol::ScriptFailure;

use crate::objects::Access;
use crate::{Computer, ComputerId, Node, WorldError};

/// Failure that prevents a base operation from returning a program result.
pub type OperationError = RouteError<WorldError>;
/// Result of a base operation.
pub type Result<T> = std::result::Result<T, OperationError>;

/// Looks up the computer referenced by `access`.
pub(crate) async fn computer(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
) -> Result<Option<Computer>> {
    Ok(world
        .module::<crate::CoreModule>()
        .networks
        .topology
        .computer(access.computer)
        .await?)
}

/// Looks up the node for `file`, returning `None` after removal.
pub(crate) async fn node(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &crate::FileRef,
) -> Result<Option<Node>> {
    Ok(world
        .module::<crate::CoreModule>()
        .files
        .lookup
        .node(file.access.computer, file.node)
        .await?)
}

/// Requires network access from the player's computer for terminals in `terminal_computers`
/// (`Computer.IsNetworkActive` of `GreyInterpreter.GetPlayerComputer`).
pub async fn require_connection(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    terminal_computers: &[ComputerId],
) -> Result<()> {
    if world
        .module::<crate::CoreModule>()
        .networks
        .reach
        .is_network_active(
            world.module::<crate::CoreModule>().world.clone().player(),
            terminal_computers,
        )
        .await?
    {
        Ok(())
    } else {
        Err(OperationError::Script(ScriptFailure::NoConnection))
    }
}

/// Registers every core route in a driver-independent table.
pub fn register<W: greyhack_world_router::WorldContext<Computer = crate::ComputerId> + 'static>(
    routes: &greyhack_world_router::RouteTable,
) {
    admin::register::<W>(routes);
    computer::register::<W>(routes);
    file::register::<W>(routes);
    router::register::<W>(routes);
}
