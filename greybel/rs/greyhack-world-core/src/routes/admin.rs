//! Core-owned `admin.so` operations for computers, credentials, and networks.

use greyhack_world_programs::protocol::{
    AdminAddNetCard, AdminDisableNetwork, AdminLogin, AdminNetworks, AdminRegisterPassword,
};
use greyhack_world_router::{RouteContext, RouteTable};

use crate::{ComputerId, NetCard};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = crate::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, crate::WorldError>(
        async move |ctx: &RouteContext<'_, W>, _call, message: AdminLogin<'static>| {
            login(
                &*ctx.modules,
                &message.public_ip,
                &message.lan_ip,
                &message.user,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async move |ctx: &RouteContext<'_, W>, _call, message: AdminAddNetCard<'static>| {
            let world = &*ctx.modules;
            let Some(computer) = world
                .module::<crate::CoreModule>()
                .networks
                .routers
                .computer_at(&message.public_ip, &message.lan_ip)
                .await?
            else {
                return Ok(false);
            };
            let card = NetCard::new(message.wifi, message.monitor_support);

            Ok(world
                .module::<crate::CoreModule>()
                .networks
                .net_cards
                .add_net_card(computer, &card)
                .await?)
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async move |ctx: &RouteContext<'_, W>, _call, message: AdminDisableNetwork<'static>| {
            Ok(ctx
                .modules
                .module::<crate::CoreModule>()
                .networks
                .settings
                .disable_network(&message.public_ip, message.disabled)
                .await?)
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async move |ctx: &RouteContext<'_, W>, _call, _message: AdminNetworks| {
            Ok(ctx
                .modules
                .module::<crate::CoreModule>()
                .networks
                .routers
                .public_addresses()
                .await?)
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async move |ctx: &RouteContext<'_, W>, _call, message: AdminRegisterPassword<'static>| {
            Ok(ctx
                .modules
                .module::<crate::CoreModule>()
                .computers
                .users
                .register_password(&message.password)
                .await?)
        },
    );
}

/// Opens a session for `user` on the addressed computer unless it is missing or already open.
async fn login(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    public_ip: &str,
    lan_ip: &str,
    user: &str,
) -> super::Result<bool> {
    let Some(id) = computer_at(world, public_ip, lan_ip).await? else {
        return Ok(false);
    };

    Ok(world
        .module::<crate::CoreModule>()
        .computers
        .users
        .open_new_session(id, user)
        .await?)
}

pub(crate) async fn computer_at(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    public_ip: &str,
    lan_ip: &str,
) -> super::Result<Option<ComputerId>> {
    Ok(world
        .module::<crate::CoreModule>()
        .networks
        .routers
        .computer_at(public_ip, lan_ip)
        .await?)
}
