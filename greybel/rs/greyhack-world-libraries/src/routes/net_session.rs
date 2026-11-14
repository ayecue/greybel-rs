//! `netSession` operations (`NetSessionIntrinsics`) on service libraries and routers.

use greyhack_world_router::RouteTable;

use crate::objects::{LibraryRef, MetaLibHandle, MetaLibRef, NetSessionRef, ObjectTableExt};
use crate::routes::Result;
use crate::routes::protocol::{
    NetSessionDumpLib, NetSessionGetNumConnGateway, NetSessionGetNumPortforward,
    NetSessionGetNumUsers, NetSessionIsAnyActiveUser, NetSessionIsRootActiveUser,
};
use crate::{ComputerId, RouterId};

pub(crate) fn register<W: crate::LibraryContext + 'static>(routes: &RouteTable) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: NetSessionDumpLib| {
            let world = &*ctx.modules;
            let session = ctx
                .trees
                .objects(call.tree)
                .net_session(message.net_session)?
                .clone();
            let library = match session {
                NetSessionRef::Service { file, origin } => MetaLibRef::File(LibraryRef {
                    access: file.access,
                    path: file.path,
                    origin,
                }),
                NetSessionRef::Router(kernel) => MetaLibRef::Kernel(kernel),
            };

            Ok(dump_lib(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .net_session(message.net_session)?,
            )
            .await?
            .then(|| ctx.trees.objects(call.tree).insert(MetaLibHandle(library))))
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: NetSessionGetNumUsers| {
            let world = &*ctx.modules;

            get_num_users(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .net_session(message.net_session)?,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: NetSessionGetNumPortforward| {
            let world = &*ctx.modules;

            get_num_portforward(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .net_session(message.net_session)?,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: NetSessionGetNumConnGateway| {
            let world = &*ctx.modules;

            get_num_conn_gateway(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .net_session(message.net_session)?,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: NetSessionIsAnyActiveUser| {
            let world = &*ctx.modules;

            is_any_active_user(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .net_session(message.net_session)?,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: NetSessionIsRootActiveUser| {
            let world = &*ctx.modules;

            is_root_active_user(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .net_session(message.net_session)?,
            )
            .await
        },
    );
}

/// Uses `router` for router sessions and `computer` for service sessions. Returns `None` after the
/// session computer is removed.
///
/// # Errors
///
/// Returns errors from database reads.
async fn count(
    world: &impl crate::LibraryContext,
    session: &NetSessionRef,
    router: impl AsyncFnOnce(RouterId) -> Result<usize>,
    computer: impl AsyncFnOnce(ComputerId) -> Result<usize>,
) -> Result<Option<u32>> {
    let count = match session {
        NetSessionRef::Router(kernel) => Some(router(kernel.router).await?),
        NetSessionRef::Service { .. } => match world
            .libraries()
            .metaxploit
            .session_computer(session)
            .await?
        {
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
pub async fn dump_lib(world: &impl crate::LibraryContext, session: &NetSessionRef) -> Result<bool> {
    let exists = match session {
        NetSessionRef::Service { file, .. } => world
            .core()
            .files
            .lookup
            .file(file.access.computer, &file.path, false)
            .await?
            .is_some(),
        NetSessionRef::Router(kernel) => world
            .core()
            .networks
            .routers
            .kernel_library(kernel.router)
            .await?
            .is_some(),
    };

    Ok(exists)
}

/// Counts user accounts on the service computer.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn get_num_users(
    world: &impl crate::LibraryContext,
    session: &NetSessionRef,
) -> Result<Option<u32>> {
    let Some(computer) = world
        .libraries()
        .metaxploit
        .session_computer(session)
        .await?
    else {
        return Ok(None);
    };
    let count = world.core().computers.users.users(computer).await?.len();

    Ok(Some(u32::try_from(count).unwrap_or(u32::MAX)))
}

/// Counts router forwards or forwards targeting the service computer.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn get_num_portforward(
    world: &impl crate::LibraryContext,
    session: &NetSessionRef,
) -> Result<Option<u32>> {
    count(
        world,
        session,
        async |router| {
            Ok(world
                .core()
                .networks
                .routers
                .router_port_forwards(router)
                .await?)
        },
        async |computer| {
            Ok(world
                .core()
                .networks
                .routers
                .port_forwards_to(computer)
                .await?)
        },
    )
    .await
}

/// Counts devices using the session router or service computer as gateway.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn get_num_conn_gateway(
    world: &impl crate::LibraryContext,
    session: &NetSessionRef,
) -> Result<Option<u32>> {
    count(
        world,
        session,
        async |router| {
            Ok(world
                .core()
                .networks
                .routers
                .router_gateway_connections(router)
                .await?)
        },
        async |computer| {
            Ok(world
                .core()
                .networks
                .routers
                .gateway_connections(computer)
                .await?)
        },
    )
    .await
}

/// Checks for any active user on the service computer.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn is_any_active_user(
    world: &impl crate::LibraryContext,
    session: &NetSessionRef,
) -> Result<Option<bool>> {
    let Some(computer) = world
        .libraries()
        .metaxploit
        .session_computer(session)
        .await?
    else {
        return Ok(None);
    };

    Ok(Some(
        world
            .core()
            .computers
            .users
            .is_any_active_user(computer)
            .await?,
    ))
}

/// Checks for an active root user on the service computer.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn is_root_active_user(
    world: &impl crate::LibraryContext,
    session: &NetSessionRef,
) -> Result<Option<bool>> {
    let Some(computer) = world
        .libraries()
        .metaxploit
        .session_computer(session)
        .await?
    else {
        return Ok(None);
    };

    Ok(Some(
        world
            .core()
            .computers
            .users
            .is_root_active_user(computer)
            .await?,
    ))
}
