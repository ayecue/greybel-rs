//! Operations on router objects (`RouterIntrinsics`).

use greyhack_world_router::protocol::{
    DevicePortList, FirewallEntry, Port, PortService, RouterBssidName, RouterDevicePorts,
    RouterDevicesLanIp, RouterEssidName, RouterFirewallRules, RouterKernelVersion, RouterLocalIp,
    RouterNetSpeed, RouterPingPort, RouterPortInfo, RouterPublicIp, RouterUsedPorts, ScriptFailure,
};
use greyhack_world_router::{RouteContext, RouteTable};

use super::port::snapshot;
use crate::objects::ObjectTableExt;
use crate::routes::conversions::IntoProtocol;
use crate::routes::{OperationError, Result};
use crate::{PortLookup, Router, RouterId, TablePort};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = crate::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: RouterPublicIp| {
            let world = &*ctx.modules;

            on_router(
                world,
                ctx.trees.objects(call.tree).router(message.router)?,
                |router| router.public_ip.clone(),
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: RouterLocalIp| {
            let world = &*ctx.modules;

            on_router(
                world,
                ctx.trees.objects(call.tree).router(message.router)?,
                |router| router.local_ip.clone(),
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: RouterEssidName| {
            let world = &*ctx.modules;

            Ok(world
                .module::<crate::CoreModule>()
                .networks
                .routers
                .essid_name(ctx.trees.objects(call.tree).router(message.router)?)
                .await?)
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: RouterBssidName| {
            let world = &*ctx.modules;

            Ok(world
                .module::<crate::CoreModule>()
                .networks
                .routers
                .bssid_name(ctx.trees.objects(call.tree).router(message.router)?)
                .await?)
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: RouterKernelVersion| {
            let world = &*ctx.modules;

            Ok(world
                .module::<crate::CoreModule>()
                .networks
                .routers
                .kernel_version(ctx.trees.objects(call.tree).router(message.router)?)
                .await?)
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: RouterNetSpeed| {
            let world = &*ctx.modules;

            Ok(world
                .module::<crate::CoreModule>()
                .networks
                .settings
                .net_speed(ctx.trees.objects(call.tree).router(message.router)?)
                .await?)
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: RouterPortInfo<'static>| {
            let world = &*ctx.modules;

            port_info(
                world,
                ctx.trees.objects(call.tree).router(message.router)?,
                &message.lan_ip,
                message.internal_port,
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: RouterFirewallRules| {
            let world = &*ctx.modules;

            firewall_rules(world, ctx.trees.objects(call.tree).router(message.router)?).await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: RouterDevicesLanIp| {
            let world = &*ctx.modules;

            devices_lan_ip(world, ctx.trees.objects(call.tree).router(message.router)?).await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: RouterUsedPorts| {
            let world = &*ctx.modules;

            Ok(world
                .module::<crate::CoreModule>()
                .networks
                .routers
                .visible_table_ports(ctx.trees.objects(call.tree).router(message.router)?)
                .await?
                .map(|ports| ports.iter().map(table_snapshot).collect()))
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: RouterPingPort| {
            let world = &*ctx.modules;

            Ok(world
                .module::<crate::CoreModule>()
                .networks
                .routers
                .table_port(
                    ctx.trees.objects(call.tree).router(message.router)?,
                    message.port,
                )
                .await?
                .as_ref()
                .map(table_snapshot))
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: RouterDevicePorts<'static>| {
            let world = &*ctx.modules;
            let computer = ctx
                .trees
                .objects(call.tree)
                .session(
                    ctx.session(call)
                        .ok_or(OperationError::Script(ScriptFailure::ProgramEnded))?,
                )?
                .computer;

            Ok(
                match world
                    .module::<crate::CoreModule>()
                    .networks
                    .routers
                    .device_ports(
                        ctx.trees.objects(call.tree).router(message.router)?,
                        &message.address,
                        computer,
                    )
                    .await?
                {
                    PortLookup::Missing => DevicePortList::Null,
                    PortLookup::Unreachable => DevicePortList::Unreachable,
                    PortLookup::Ports(ports) => {
                        DevicePortList::Ports(ports.iter().map(table_snapshot).collect())
                    }
                },
            )
        },
    );
}

/// Applies `read` to `router` if it exists; otherwise returns `None`.
async fn on_router<T>(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    router: RouterId,
    read: impl FnOnce(&Router) -> T,
) -> Result<Option<T>> {
    Ok(world
        .module::<crate::CoreModule>()
        .networks
        .routers
        .router(router)
        .await?
        .map(|router| read(&router)))
}

/// Looks up the service and library version at `lan_ip`/`internal_port` through `router`
/// (`Router.GetPortVersion`).
async fn port_info(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    router: RouterId,
    lan_ip: &str,
    internal_port: u16,
) -> Result<Option<PortService>> {
    Ok(world
        .module::<crate::CoreModule>()
        .networks
        .routers
        .port_service(router, lan_ip, internal_port)
        .await?
        .map(|(service, version)| PortService {
            service: service.name().to_owned(),
            version,
        }))
}

/// Lists the router's firewall rules.
async fn firewall_rules(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    router: RouterId,
) -> Result<Option<Vec<FirewallEntry>>> {
    if world
        .module::<crate::CoreModule>()
        .networks
        .routers
        .router(router)
        .await?
        .is_none()
    {
        return Ok(None);
    }

    Ok(Some(
        world
            .module::<crate::CoreModule>()
            .networks
            .router_config
            .firewall_rules(router)
            .await?
            .into_iter()
            .map(IntoProtocol::into_protocol)
            .collect(),
    ))
}

/// Lists the router's LAN (`Router.GetComputersIPs`): reachable computers using it as gateway, followed by
/// all network routers and switches.
async fn devices_lan_ip(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    router: RouterId,
) -> Result<Option<Vec<String>>> {
    let Some(router) = world
        .module::<crate::CoreModule>()
        .networks
        .routers
        .router(router)
        .await?
    else {
        return Ok(None);
    };

    Ok(world
        .module::<crate::CoreModule>()
        .networks
        .lan
        .lan(router.network)
        .await?
        .map(|lan| lan.router_lan(&router.local_ip, true)))
}

/// Converts a router table entry to protocol form using its public port number.
fn table_snapshot(port: &TablePort) -> Port {
    Port {
        number: port.number,
        internal_port: port.port.number,
        ..snapshot(&port.port, &port.lan_ip)
    }
}
