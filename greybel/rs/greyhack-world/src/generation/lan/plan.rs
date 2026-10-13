use super::DevicePlan;
use crate::ForwardLayout;
use crate::generation::missions::MissionPlacement;

/// LAN plan containing devices in main-router-first order and main-router forwards.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LanPlan {
    pub devices: Vec<DevicePlan>,
    pub forwards: Vec<ForwardLayout>,
    /// Where the procedural mission of the network put its computers.
    pub mission: Option<MissionPlacement>,
}
