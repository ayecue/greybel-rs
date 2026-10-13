use greyhack_behaviour::DeviceKind;

use super::ServicePlan;

/// Planned computer (`LanPcInfo`): standard computer, server, camera, or smart appliance.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ComputerPlan {
    pub kind: DeviceKind,
    pub local_ip: String,
    /// Empty while a hub computer awaits a router. `0.0.0.0` marks a private-subnet gateway whose
    /// router is not yet planned.
    pub gateway: String,
    pub services: Vec<ServicePlan>,
}
