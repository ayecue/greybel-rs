use super::LanPort;
use crate::{DeviceKind, FirewallRule};

/// LAN computer or network device.
#[derive(Debug, Clone, PartialEq)]
pub struct LanNode<K> {
    /// Caller-defined node identifier.
    pub key: K,
    pub kind: DeviceKind,
    /// Local address; empty for hubs, which have none (`HubNet`).
    pub local_ip: String,
    /// Computer gateway (`LanPcInfo.gateway`); ignored for devices.
    pub gateway: String,
    /// Marks a currently unreachable computer (`LanPcInfo.IsHidden`), such as a smartphone whose owner is away from work.
    pub hidden: bool,
    /// Device firewall rules (`Device.firewallRules`).
    pub firewall: Vec<FirewallRule>,
    /// Router port table (`RouterConfigOS.puertos`), preserving the order of local ports and forwards.
    pub ports: Vec<LanPort>,
}

impl<K> LanNode<K> {
    #[must_use]
    pub fn new(key: K, kind: DeviceKind, local_ip: &str) -> Self {
        Self {
            key,
            kind,
            local_ip: local_ip.to_owned(),
            gateway: String::new(),
            hidden: false,
            firewall: Vec::new(),
            ports: Vec::new(),
        }
    }

    /// Checks whether this node is a router, switch, or hub rather than a computer.
    #[must_use]
    pub const fn is_device(&self) -> bool {
        is_device(self.kind)
    }
}

/// Checks whether `kind` is a branching `Device` (router, switch, or hub) rather than a `LanPcInfo` computer.
#[must_use]
pub const fn is_device(kind: DeviceKind) -> bool {
    matches!(
        kind,
        DeviceKind::Router | DeviceKind::Switch | DeviceKind::Hub
    )
}

/// Ranks `kind` among children as computers, hubs and switches, then routers, matching game searches
/// (`Device.lanPcInfos`, `hubs`, `routers`).
pub(super) const fn rank(kind: DeviceKind) -> u8 {
    match kind {
        DeviceKind::Router => 2,
        DeviceKind::Switch | DeviceKind::Hub => 1,
        _ => 0,
    }
}
