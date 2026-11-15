//! Events published by NPC administrator activity.

use greyhack_world_core::NetworkId;
use greyhack_world_effects::Event;

/// An administrator arrived at a company's network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdminArrival {
    /// Network whose administrator arrived.
    pub network: NetworkId,
}

impl Event for AdminArrival {}

/// An administrator's absence penalties apply to a company's network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdminAbsence {
    /// Network whose administrator is absent.
    pub network: NetworkId,
}

impl Event for AdminAbsence {}

/// An NPC joined or left a company's staff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcJobChange {
    /// Network whose company staff changed.
    pub network: NetworkId,
}

impl Event for NpcJobChange {}
