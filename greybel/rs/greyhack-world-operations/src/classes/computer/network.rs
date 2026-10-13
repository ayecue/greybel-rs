//! Computer ports, addresses, network cards, Wi-Fi, and Ethernet (`ComputerIntrinsics`).

use greyhack_world::{ActiveCard, ComputerId, WifiRequest, WifiScan, World};

use crate::Result;
use crate::classes::port::snapshot;
use crate::classes::{computer, require_connection};
use crate::connection::objects::Access;
use crate::protocol::{
    ActiveCard as CardKind, EthernetSettings, NetworkDevice, NetworkSwitch, Port, ScriptFailure,
    WifiNetwork, WifiScan as Scan,
};

/// Lists ports visible outside the computer.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn get_ports(world: &World, access: &Access) -> Result<Option<Vec<Port>>> {
    let Some(computer) = computer(world, access).await? else {
        return Ok(None);
    };
    let ports = world
        .ports(access.computer)
        .await?
        .into_iter()
        .filter(|port| port.visible)
        .map(|port| snapshot(&port, &computer.local_ip))
        .collect();

    Ok(Some(ports))
}

/// Returns the active card type, or `None` after computer removal.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn active_net_card(world: &World, access: &Access) -> Result<Option<CardKind>> {
    Ok(world
        .active_card_kind(access.computer)
        .await?
        .map(|card| match card {
            ActiveCard::None => CardKind::None,
            ActiveCard::Wifi => CardKind::Wifi,
            ActiveCard::Ethernet => CardKind::Ethernet,
        }))
}

/// Lists the network cards of the computer.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn network_devices(world: &World, access: &Access) -> Result<Option<Vec<NetworkDevice>>> {
    Ok(world
        .network_devices(access.computer)
        .await?
        .map(|devices| {
            devices
                .into_iter()
                .map(|device| NetworkDevice {
                    name: device.name,
                    model: device.model,
                    monitor: device.monitor,
                })
                .collect()
        }))
}

/// Lists networks visible to `device`, or why none can be.
///
/// # Errors
///
/// Returns [`ScriptFailure::WifiCardInactive`] for a disabled card and
/// search failures use [`OperationError::World`](crate::OperationError::World).
pub async fn wifi_networks(world: &World, access: &Access, device: &str) -> Result<Option<Scan>> {
    Ok(match world.scan_wifi(access.computer, device).await? {
        None => None,
        Some(WifiScan::NoDevice) => Some(Scan::NoDevice),
        Some(WifiScan::NotWifi) => Some(Scan::NotWifi),
        Some(WifiScan::Inactive) => return Err(ScriptFailure::WifiCardInactive.into()),
        Some(WifiScan::Networks(networks)) => Some(Scan::Networks(
            networks
                .into_iter()
                .map(|(bssid, power, essid)| WifiNetwork {
                    bssid,
                    power,
                    essid,
                })
                .collect(),
        )),
    })
}

/// Connects `device` to Wi-Fi `bssid`/`essid` with `password`. `wait` reports a reconnect attempted too
/// soon.
///
/// # Errors
///
/// Returns errors from database searches.
pub async fn connect_wifi(
    world: &mut World,
    access: &Access,
    request: &WifiRequest<'_>,
    wait: bool,
) -> Result<NetworkSwitch> {
    Ok(world
        .join_wifi(access.computer, &access.user, request, wait)
        .await?
        .into())
}

/// Applies Ethernet `settings` in `ComputerIntrinsics.connect_ethernet` order. `wait` marks a network
/// switch attempted less than one second after the previous switch.
///
/// # Errors
///
/// Returns [`ScriptFailure::InvalidArguments`] for missing settings, [`ScriptFailure::NoConnection`] when
/// offline, and [`OperationError::World`](crate::OperationError::World) for search failures.
pub async fn connect_ethernet(
    world: &mut World,
    access: &Access,
    terminal_computers: &[ComputerId],
    settings: Option<&EthernetSettings>,
    wait: bool,
) -> Result<NetworkSwitch> {
    let Some(settings) = settings else {
        return Err(ScriptFailure::InvalidArguments.into());
    };

    require_connection(world, terminal_computers).await?;

    Ok(world
        .join_ethernet(
            access.computer,
            &access.user,
            (&settings.device, &settings.address, &settings.gateway),
            wait,
        )
        .await?
        .into())
}
