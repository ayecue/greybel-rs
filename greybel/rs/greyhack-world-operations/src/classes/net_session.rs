//! `netSession` operations (`NetSessionIntrinsics`) on service libraries and routers.

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_world::{ComputerId, Flood, FloodWarning, RouterId, World};

use crate::Result;
use crate::connection::Connection;
use crate::connection::objects::{LibraryRef, MetaLibRef, NetSessionRef, Object};
use crate::protocol::{
    FloodResult, FloodWarning as WarningKind, GameServer, NetSessionDumpLib,
    NetSessionFloodConnection, NetSessionGetNumConnGateway, NetSessionGetNumPortforward,
    NetSessionGetNumUsers, NetSessionIsAnyActiveUser, NetSessionIsRootActiveUser,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: NetSessionDumpLib, _| {
            let session = objects.net_session(message.net_session)?.clone();
            let library = match session {
                NetSessionRef::Service { file, origin } => MetaLibRef::File(LibraryRef {
                    access: file.access,
                    path: file.path,
                    origin,
                }),
                NetSessionRef::Router(kernel) => MetaLibRef::Kernel(kernel),
            };

            Ok(dump_lib(world, objects.net_session(message.net_session)?)
                .await?
                .then(|| objects.add(Object::MetaLib(library))))
        },
    );
    connection.read(
        channel,
        async |world, objects, message: NetSessionGetNumUsers, _| {
            get_num_users(world, objects.net_session(message.net_session)?).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: NetSessionGetNumPortforward, _| {
            get_num_portforward(world, objects.net_session(message.net_session)?).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: NetSessionGetNumConnGateway, _| {
            get_num_conn_gateway(world, objects.net_session(message.net_session)?).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: NetSessionIsAnyActiveUser, _| {
            is_any_active_user(world, objects.net_session(message.net_session)?).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: NetSessionIsRootActiveUser, _| {
            is_root_active_user(world, objects.net_session(message.net_session)?).await
        },
    );
    connection.operate(
        channel,
        async |state, connection, message: NetSessionFloodConnection| {
            state
                .cycle(async |world_state| {
                    state.session(connection.id, message.program)?;

                    let (world, objects) = world_state.of(connection.id);
                    let session = objects.net_session(message.net_session)?.clone();

                    let Some(computer) = session_computer(world, &session).await? else {
                        return Ok::<_, crate::OperationError>(FloodResult::Gone);
                    };
                    let attempt = world.flood_from(computer, session.origin()).await?;
                    let address = attempt.address;
                    let warning = match attempt.warning {
                        None => WarningKind::None,
                        Some(FloodWarning::SuperAdmin) => WarningKind::SuperAdmin,
                        Some(FloodWarning::CrownShield) => WarningKind::CrownShield,
                    };

                    Ok(match attempt.flood {
                        Flood::Active(work) => {
                            // Flood work continues until program termination cancels it (`WaitResultDownload`).
                            drop(state.defer_job(connection.id, message.program, work));

                            FloodResult::Active { address, warning }
                        }
                        Flood::Shield => FloodResult::Active { address, warning },
                        Flood::Reboot => FloodResult::Reboot { address, warning },
                    })
                })
                .await
        },
    );
}

/// Returns the service or router computer if it still exists.
async fn session_computer(world: &World, session: &NetSessionRef) -> Result<Option<ComputerId>> {
    let id = match session {
        NetSessionRef::Service { file, .. } => file.access.computer,
        NetSessionRef::Router(kernel) => kernel.router.computer(),
    };

    Ok(world.computer(id).await?.map(|_| id))
}

/// Uses `router` for router sessions and `computer` for service sessions. Returns `None` after the
/// session computer is removed.
///
/// # Errors
///
/// Returns errors from database reads.
async fn count(
    world: &World,
    session: &NetSessionRef,
    router: impl AsyncFnOnce(RouterId) -> Result<usize>,
    computer: impl AsyncFnOnce(ComputerId) -> Result<usize>,
) -> Result<Option<u32>> {
    let count = match session {
        NetSessionRef::Router(kernel) => Some(router(kernel.router).await?),
        NetSessionRef::Service { .. } => match session_computer(world, session).await? {
            Some(id) => Some(computer(id).await?),
            None => None,
        },
    };

    Ok(count.map(|count| u32::try_from(count).unwrap_or(u32::MAX)))
}

/// Checks that the service library or router `kernel_router.so` still exists.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn dump_lib(world: &World, session: &NetSessionRef) -> Result<bool> {
    let exists = match session {
        NetSessionRef::Service { file, .. } => world
            .file(file.access.computer, &file.path, false)
            .await?
            .is_some(),
        NetSessionRef::Router(kernel) => world.kernel_library(kernel.router).await?.is_some(),
    };

    Ok(exists)
}

/// Counts user accounts on the service computer.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn get_num_users(world: &World, session: &NetSessionRef) -> Result<Option<u32>> {
    let Some(computer) = session_computer(world, session).await? else {
        return Ok(None);
    };
    let count = world.users(computer).await?.len();

    Ok(Some(u32::try_from(count).unwrap_or(u32::MAX)))
}

/// Counts router forwards or forwards targeting the service computer.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn get_num_portforward(world: &World, session: &NetSessionRef) -> Result<Option<u32>> {
    count(
        world,
        session,
        async |router| Ok(world.router_port_forwards(router).await?),
        async |computer| Ok(world.port_forwards_to(computer).await?),
    )
    .await
}

/// Counts devices using the session router or service computer as gateway.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn get_num_conn_gateway(world: &World, session: &NetSessionRef) -> Result<Option<u32>> {
    count(
        world,
        session,
        async |router| Ok(world.router_gateway_connections(router).await?),
        async |computer| Ok(world.gateway_connections(computer).await?),
    )
    .await
}

/// Checks for any active user on the service computer.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn is_any_active_user(world: &World, session: &NetSessionRef) -> Result<Option<bool>> {
    let Some(computer) = session_computer(world, session).await? else {
        return Ok(None);
    };

    Ok(Some(world.is_any_active_user(computer).await?))
}

/// Checks for an active root user on the service computer.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn is_root_active_user(world: &World, session: &NetSessionRef) -> Result<Option<bool>> {
    let Some(computer) = session_computer(world, session).await? else {
        return Ok(None);
    };

    Ok(Some(world.is_root_active_user(computer).await?))
}
