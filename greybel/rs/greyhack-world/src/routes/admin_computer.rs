//! Single-computer `admin.so` operations (`adminComputer`), addressed by public and LAN IP. Includes the
//! game's UI/service SQL, CCTV, and ADB channels.

use greyhack_world_router::RouteTable;

use crate::objects::{Access, Object, ObjectTableExt};
use crate::routes::Result;
use crate::routes::conversions::{
    AdminAddNetCard, AdminLogin, AdminSetLibraryVersion, AdminSetPortLibraryVersion, AdminShell,
};
use crate::{ComputerId, NetCard, World};

pub(crate) fn register(routes: &RouteTable) {
    routes.read::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    call,
                    message: AdminShell<'static>| {
            let world = &*ctx.modules;

            Ok(
                shell(world, &message.public_ip, &message.lan_ip, &message.user)
                    .await?
                    .map(|computer| {
                        ctx.trees.objects(call.tree).add(Object::Shell(Access {
                            computer,
                            user: message.user.to_string(),
                            ftp: false,
                            prev_ip: String::new(),
                            bounces: 0,
                        }))
                    }),
            )
        },
    );
    routes.change::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    message: AdminLogin<'static>| {
            let world = &*ctx.modules;

            login(world, &message.public_ip, &message.lan_ip, &message.user).await
        },
    );
    routes.change::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    message: AdminAddNetCard<'static>| {
            let world = &*ctx.modules;
            let Some(computer) = world
                .modules
                .core
                .networks
                .routers
                .computer_at(&message.public_ip, &message.lan_ip)
                .await?
            else {
                return Ok(false);
            };
            let card = NetCard::new(message.wifi, message.monitor_support);

            Ok(world
                .modules
                .core
                .networks
                .net_cards
                .add_net_card(computer, &card)
                .await?)
        },
    );
    routes.change::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    message: AdminSetLibraryVersion<'static>| {
            let world = &*ctx.modules;
            let Some(computer) = world
                .modules
                .core
                .networks
                .routers
                .computer_at(&message.public_ip, &message.lan_ip)
                .await?
            else {
                return Ok(false);
            };

            Ok(world
                .modules
                .core
                .computers
                .hardware
                .set_library_version(computer, &message.path, &message.version)
                .await?)
        },
    );
    routes.change::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    message: AdminSetPortLibraryVersion<'static>| {
            let world = &*ctx.modules;
            let Some(computer) = world
                .modules
                .core
                .networks
                .routers
                .computer_at(&message.public_ip, &message.lan_ip)
                .await?
            else {
                return Ok(false);
            };

            Ok(world
                .modules
                .core
                .computers
                .hardware
                .set_port_library_version(computer, message.port, &message.version)
                .await?)
        },
    );
}

/// Finds the computer at `lan_ip` behind `public_ip` only if it contains `user`.
///
/// # Errors
///
/// Returns errors from database searches.
pub async fn shell(
    world: &World,
    public_ip: &str,
    lan_ip: &str,
    user: &str,
) -> Result<Option<ComputerId>> {
    let Some(id) = world
        .modules
        .core
        .networks
        .routers
        .computer_at(public_ip, lan_ip)
        .await?
    else {
        return Ok(None);
    };
    let has_user = world
        .modules
        .core
        .computers
        .users
        .user(id, user)
        .await?
        .is_some();

    Ok(has_user.then_some(id))
}

/// Opens a session for `user` on the computer at `lan_ip` behind `public_ip` unless missing or already open.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn login(world: &World, public_ip: &str, lan_ip: &str, user: &str) -> Result<bool> {
    let Some(id) = world
        .modules
        .core
        .networks
        .routers
        .computer_at(public_ip, lan_ip)
        .await?
    else {
        return Ok(false);
    };

    Ok(world
        .modules
        .core
        .computers
        .users
        .open_new_session(id, user)
        .await?)
}
