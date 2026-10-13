//! Operations on router objects (`RouterIntrinsics`).

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_world::{PortLookup, Router, RouterId, TablePort, World};

use super::port::snapshot;
use crate::Result;
use crate::connection::Connection;
use crate::protocol::{
    DevicePortList, FirewallEntry, GameServer, Port, PortService, RouterBssidName,
    RouterDevicePorts, RouterDevicesLanIp, RouterEssidName, RouterFirewallRules,
    RouterKernelVersion, RouterLocalIp, RouterNetSpeed, RouterPingPort, RouterPortInfo,
    RouterPublicIp, RouterUsedPorts,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: RouterPublicIp, _| {
            on_router(world, *objects.router(message.router)?, |router| {
                router.public_ip.clone()
            })
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: RouterLocalIp, _| {
            on_router(world, *objects.router(message.router)?, |router| {
                router.local_ip.clone()
            })
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: RouterEssidName, _| {
            Ok(world.essid_name(*objects.router(message.router)?).await?)
        },
    );
    connection.read(
        channel,
        async |world, objects, message: RouterBssidName, _| {
            Ok(world.bssid_name(*objects.router(message.router)?).await?)
        },
    );
    connection.read(
        channel,
        async |world, objects, message: RouterKernelVersion, _| {
            Ok(world
                .kernel_version(*objects.router(message.router)?)
                .await?)
        },
    );
    connection.read(
        channel,
        async |world, objects, message: RouterNetSpeed, _| {
            Ok(world.net_speed(*objects.router(message.router)?).await?)
        },
    );
    connection.read(
        channel,
        async |world, objects, message: RouterPortInfo<'static>, _| {
            port_info(
                world,
                *objects.router(message.router)?,
                &message.lan_ip,
                message.internal_port,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: RouterFirewallRules, _| {
            firewall_rules(world, *objects.router(message.router)?).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: RouterDevicesLanIp, _| {
            devices_lan_ip(world, *objects.router(message.router)?).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: RouterUsedPorts, _| {
            Ok(world
                .visible_table_ports(*objects.router(message.router)?)
                .await?
                .map(|ports| ports.iter().map(table_snapshot).collect()))
        },
    );
    connection.read(
        channel,
        async |world, objects, message: RouterPingPort, _| {
            Ok(world
                .table_port(*objects.router(message.router)?, message.port)
                .await?
                .as_ref()
                .map(table_snapshot))
        },
    );
    connection.read(
        channel,
        async |world, objects, message: RouterDevicePorts<'static>, caller| {
            let computer = objects.session(caller.session)?.computer;

            Ok(
                match world
                    .device_ports(*objects.router(message.router)?, &message.address, computer)
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
    world: &World,
    router: RouterId,
    read: impl FnOnce(&Router) -> T,
) -> Result<Option<T>> {
    Ok(world.router(router).await?.map(|router| read(&router)))
}

/// Looks up the service and library version at `lan_ip`/`internal_port` through `router`
/// (`Router.GetPortVersion`).
async fn port_info(
    world: &World,
    router: RouterId,
    lan_ip: &str,
    internal_port: u16,
) -> Result<Option<PortService>> {
    Ok(world
        .port_service(router, lan_ip, internal_port)
        .await?
        .map(|(service, version)| PortService {
            service: service.name().to_owned(),
            version,
        }))
}

/// Lists the router's firewall rules.
async fn firewall_rules(world: &World, router: RouterId) -> Result<Option<Vec<FirewallEntry>>> {
    if world.router(router).await?.is_none() {
        return Ok(None);
    }

    Ok(Some(
        world
            .firewall_rules(router)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    ))
}

/// Lists the router's LAN (`Router.GetComputersIPs`): reachable computers using it as gateway, followed by
/// all network routers and switches.
async fn devices_lan_ip(world: &World, router: RouterId) -> Result<Option<Vec<String>>> {
    let Some(router) = world.router(router).await? else {
        return Ok(None);
    };

    Ok(world
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
