//! Computer ports, addresses, network cards, Wi-Fi, and Ethernet (`ComputerIntrinsics`).

use greyhack_world_router::protocol::{
    ActiveCard as CardKind, EthernetSettings, NetworkDevice, NetworkSwitch, Port, ScriptFailure,
    WifiNetwork, WifiScan as Scan,
};

use crate::objects::Access;
use crate::routes::conversions::IntoProtocol as _;
use crate::routes::port::snapshot;
use crate::routes::{OperationError, Result, computer, require_connection};
use crate::{ActiveCard, ComputerId, WifiRequest, WifiScan};

/// Lists ports visible outside the computer.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn get_ports(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
) -> Result<Option<Vec<Port>>> {
    let Some(computer) = computer(world, access).await? else {
        return Ok(None);
    };
    let ports = world
        .module::<crate::CoreModule>()
        .computers
        .processes
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
pub async fn active_net_card(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
) -> Result<Option<CardKind>> {
    Ok(world
        .module::<crate::CoreModule>()
        .networks
        .net_cards
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
pub async fn network_devices(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
) -> Result<Option<Vec<NetworkDevice>>> {
    Ok(world
        .module::<crate::CoreModule>()
        .networks
        .net_cards
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
pub async fn wifi_networks(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
    device: &str,
) -> Result<Option<Scan>> {
    Ok(
        match world
            .module::<crate::CoreModule>()
            .networks
            .net_cards
            .scan_wifi(access.computer, device)
            .await?
        {
            None => None,
            Some(WifiScan::NoDevice) => Some(Scan::NoDevice),
            Some(WifiScan::NotWifi) => Some(Scan::NotWifi),
            Some(WifiScan::Inactive) => {
                return Err(OperationError::Script(ScriptFailure::WifiCardInactive));
            }
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
        },
    )
}

/// Connects `device` to Wi-Fi `bssid`/`essid` with `password`. `wait` reports a reconnect attempted too
/// soon.
///
/// # Errors
///
/// Returns errors from database searches.
pub async fn connect_wifi(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
    request: &WifiRequest<'_>,
    wait: bool,
) -> Result<NetworkSwitch> {
    Ok(world
        .module::<crate::CoreModule>()
        .networks
        .net_cards
        .join_wifi(access.computer, &access.user, request, wait)
        .await?
        .into_protocol())
}

/// Applies Ethernet `settings` in `ComputerIntrinsics.connect_ethernet` order. `wait` marks a network
/// switch attempted less than one second after the previous switch.
///
/// # Errors
///
/// Returns [`ScriptFailure::InvalidArguments`] for missing settings, [`ScriptFailure::NoConnection`] when
/// offline, and [`OperationError::World`](crate::OperationError::World) for search failures.
pub async fn connect_ethernet(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
    terminal_computers: &[ComputerId],
    settings: Option<&EthernetSettings>,
    wait: bool,
) -> Result<NetworkSwitch> {
    let Some(settings) = settings else {
        return Err(OperationError::Script(ScriptFailure::InvalidArguments));
    };

    require_connection(world, terminal_computers).await?;

    Ok(world
        .module::<crate::CoreModule>()
        .networks
        .net_cards
        .join_ethernet(
            access.computer,
            &access.user,
            (&settings.device, &settings.address, &settings.gateway),
            wait,
        )
        .await?
        .into_protocol())
}
