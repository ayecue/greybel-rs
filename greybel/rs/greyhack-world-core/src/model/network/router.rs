//! Router public addressing, Wi-Fi, connected devices, and forwarded ports.

use crate::model::{ComputerDraft, ComputerId, NetworkId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RouterId(pub u32);

impl RouterId {
    /// Computer associated with the router.
    #[must_use]
    pub const fn computer(self) -> ComputerId {
        ComputerId(self.0)
    }

    /// Returns the router for `computer`; callers must ensure the computer is a router.
    #[must_use]
    pub const fn of(computer: ComputerId) -> Self {
        Self(computer.0)
    }
}

use greyhack_behaviour::{DeviceKind, FirewallRule, NetworkKind, wifi_power};

/// Contact information returned by `whois`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    pub name: String,
    pub mail: String,
    pub phone: String,
}

/// Router or switch with shared network public address, domain, and administrator. The main router
/// accepts external connections; LAN routers and switches are its descendants (`LanRouter`) and
/// share its public address. Ports, firewall rules, kernel, reverse shells, and police traffic data
/// use separate rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Router {
    /// Router computer ID.
    pub id: RouterId,
    pub network: NetworkId,
    /// Router or switch.
    pub kind: DeviceKind,
    /// Network main router; self-referential for the main router.
    pub main: RouterId,
    pub public_ip: String,
    pub local_ip: String,
    /// Domain resolved by `nslookup` to the router public address.
    pub domain: String,
    pub contact: Option<Contact>,
    /// `None` for LAN routers without Wi-Fi (`NetworkLan.RouterNet.ConfigWifi`).
    pub essid: Option<String>,
    pub bssid: Option<String>,
    pub wifi_password: String,
    /// Map coordinates in game units (`Map.posX/posY`). Wi-Fi range is 0.5 units.
    pub map: [f32; 2],
    /// Whether the firewall is enabled. `overflow` `FirewallDisable` clears its rules and disables
    /// it.
    pub firewall_active: bool,
}

impl Router {
    /// Returns stable Wi-Fi signal strength derived from the BSSID for captures.
    #[must_use]
    pub fn wifi_power(&self) -> i32 {
        wifi_power(self.bssid.as_deref().unwrap_or_default())
    }

    /// Returns whether this is the externally reachable main router.
    #[must_use]
    pub fn is_main(&self) -> bool {
        self.main == self.id
    }
}

/// Generated network state shared by devices before database insertion.
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkDraft {
    pub kind: NetworkKind,
    /// Whether the LAN follows Grey Hack generation rules.
    pub has_plan: bool,
    pub public_ip: String,
    pub domain: String,
    pub contact: Option<Contact>,
    pub position: [f32; 2],
}

/// Router or switch draft with all associated state before database insertion.
#[derive(Debug, Clone, PartialEq)]
pub struct RouterDraft<L = crate::Library, S = ()> {
    pub essid: Option<String>,
    pub bssid: Option<String>,
    pub wifi_password: String,
    pub firewall_rules: Vec<FirewallRule>,
    pub firewall_active: bool,
    /// Router or switch computer with users, ports, and a file system containing
    /// `/lib/kernel_router.so`.
    pub computer: Box<ComputerDraft<L, S>>,
}
