use greyhack_behaviour::NetworkKind;
use greyhack_world_missions::MissionPlacement;

use super::DevicePlan;
use crate::ForwardLayout;

/// LAN plan containing devices in main-router-first order and main-router forwards.
#[derive(Debug, Clone, PartialEq)]
pub struct LanPlan {
    /// The kind the network is generated as: its own, or the one its mission sets.
    pub kind: NetworkKind,
    pub devices: Vec<DevicePlan>,
    pub forwards: Vec<ForwardLayout>,
    /// Where the procedural mission of the network put its computers.
    pub mission: Option<MissionPlacement>,
}
