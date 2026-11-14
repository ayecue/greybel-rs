use greyhack_behaviour::{DeviceKind, FirewallRule};

use super::ComputerPlan;

/// Planned router, switch, or hub.
#[derive(Debug, Clone, PartialEq)]
pub struct DevicePlan {
    pub kind: DeviceKind,
    /// Empty for hubs.
    pub local_ip: String,
    pub parent: Option<usize>,
    /// BSSID and ESSID for routers with Wi-Fi.
    pub wifi: Option<(String, String)>,
    pub firewall: Vec<FirewallRule>,
    pub computers: Vec<ComputerPlan>,
    /// Child hubs and switches, followed by routers, referenced by index.
    pub(super) hubs: Vec<usize>,
    pub(super) routers: Vec<usize>,
    /// Gateway computer address for a hub private subnet (`RouterNet.privateGateway`).
    pub(super) private_gateway: String,
}

impl DevicePlan {
    pub(super) fn new(kind: DeviceKind, local_ip: &str, parent: Option<usize>) -> Self {
        Self {
            kind,
            local_ip: local_ip.to_owned(),
            parent,
            wifi: None,
            firewall: Vec::new(),
            computers: Vec::new(),
            hubs: Vec::new(),
            routers: Vec::new(),
            private_gateway: String::new(),
        }
    }

    pub(super) const fn is_router(&self) -> bool {
        matches!(self.kind, DeviceKind::Router)
    }

    pub(super) const fn is_hub(&self) -> bool {
        matches!(self.kind, DeviceKind::Hub)
    }

    pub(super) const fn is_switch(&self) -> bool {
        matches!(self.kind, DeviceKind::Switch)
    }
}
