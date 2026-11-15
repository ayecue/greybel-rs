//! Net sessions (`MetaExploitIntrinsics.net_use`).

use greyhack_behaviour::ROOT_USER;
use greyhack_world_core::routes::require_connection;
use greyhack_world_core::{Access, ComputerId, FileRef};

use crate::objects::{KernelRef, NetSessionRef};
use crate::routes::protocol::ScriptFailure;
use crate::routes::{OperationError, Result};
use crate::{NetUseFailure, NetUseSession};

/// Opens a service session at `address:port`, or a router session when port is 0 ([`crate::Metaxploit::net_use`]).
/// Returns `None` when no session target exists.
///
/// # Errors
///
/// Returns [`crate::routes::protocol::ScriptFailure::NoConnection`] when offline and
/// [`OperationError::World`](crate::routes::OperationError::World) for search failures.
pub(super) async fn net_use(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    access: &Access,
    terminal_computers: &[ComputerId],
    address: &str,
    port: u16,
) -> Result<std::result::Result<Option<NetSessionRef>, NetUseFailure>> {
    require_connection(world, terminal_computers).await?;

    // `MetaExploitIntrinsics.net_use` hands the session's library the metaxploit computer's address as its
    // previous address.
    let origin = world
        .module::<greyhack_world_core::CoreModule>()
        .networks
        .topology
        .public_ip(access.computer)
        .await?;

    Ok(world
        .module::<crate::LibraryModule>()
        .metaxploit
        .net_use(access.computer, address, port)
        .await?
        .map(|session| {
            session.map(|session| match session {
                NetUseSession::Router { router, address } => NetSessionRef::Router(KernelRef {
                    router,
                    address,
                    origin: access.computer,
                    bounces: access.bounces,
                }),
                NetUseSession::Service(file) => NetSessionRef::Service {
                    file: FileRef {
                        access: Access {
                            computer: file.computer,
                            user: ROOT_USER.to_owned(),
                            ftp: false,
                            prev_ip: origin,
                            bounces: access.bounces,
                        },
                        node: file.id,
                        path: file.path,
                    },
                    origin: access.computer,
                },
            })
        }))
}

/// Handles `net_use` with a missing receiver after connection and address checks, then dereferences the helper.
pub(super) async fn net_use_with_optional_access(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    access: Option<&Access>,
    terminal_computers: &[ComputerId],
    address: &str,
    port: u16,
) -> Result<std::result::Result<Option<NetSessionRef>, NetUseFailure>> {
    let Some(access) = access else {
        require_connection(world, terminal_computers).await?;

        if !greyhack_behaviour::is_valid_ip(address) {
            return Ok(Err(NetUseFailure::InvalidAddress));
        }

        return Err(OperationError::Script(ScriptFailure::GameCrash));
    };

    net_use(world, access, terminal_computers, address, port).await
}
