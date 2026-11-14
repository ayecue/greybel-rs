//! Single-network `admin.so` operations addressed by public IP.

use greyhack_world_router::RouteTable;

use crate::routes::conversions::{AdminDisableNetwork, AdminRemoveNetwork, AdminSetNetSpeed};

pub(crate) fn register(routes: &RouteTable) {
    routes.change::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    message: AdminRemoveNetwork<'static>| {
            let world = &*ctx.modules;

            Ok(world
                .modules
                .economy
                .bank
                .remove_network(&message.public_ip)
                .await?)
        },
    );
    routes.change::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    message: AdminDisableNetwork<'static>| {
            let world = &*ctx.modules;

            Ok(world
                .modules
                .core
                .networks
                .settings
                .disable_network(&message.public_ip, message.disabled)
                .await?)
        },
    );
    routes.change::<_, _, crate::World, crate::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, crate::World>,
                    _call,
                    message: AdminSetNetSpeed<'static>| {
            let world = &*ctx.modules;

            Ok(world
                .modules
                .economy
                .bank
                .set_net_speed(&message.public_ip, message.net_speed)
                .await?)
        },
    );
}
