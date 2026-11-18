//! Networks and routers.

use greyhack_behaviour::NetworkKind;

pub mod failure;
pub mod router;

/// Network ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkId(pub u32);

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
    /// Whether the main router is the player's home network (`RouterConfigOS.isHomeNetwork`).
    pub home: bool,
    /// Connection speed from [`greyhack_behaviour::NET_SPEEDS`] (`NetworkLan.netSpeed`).
    pub net_speed: u8,
}
