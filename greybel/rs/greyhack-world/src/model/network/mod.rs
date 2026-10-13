//! Networks, routers, services, libraries, traffic systems, and captured packets.

use greyhack_behaviour::NetworkKind;

pub(crate) mod capture;
pub(crate) mod failure;
pub(crate) mod layout;
pub(crate) mod records;
pub(crate) mod router;
pub(crate) mod service_config;
pub(crate) mod session;
pub(crate) mod sniffer;
pub(crate) mod traffic;
pub(crate) mod weakness;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkId(pub(crate) u32);

/// Network containing routers and their connected devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Network {
    pub id: NetworkId,
    pub kind: NetworkKind,
    /// Whether the LAN uses Grey Hack generation rules (`NetworkLan`) and can be restored by its
    /// administrator. Fixed-size generated networks have no plan.
    pub has_plan: bool,
    /// Whether the network is disabled for nonpayment (`NetworkLan.disabledNetwork`), rejecting all
    /// connections.
    pub disabled: bool,
    /// Connection speed from [`greyhack_behaviour::NET_SPEEDS`] (`NetworkLan.netSpeed`).
    pub net_speed: u8,
}
