use greyhack_behaviour::ServiceId;

/// Service and port assigned to a planned computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServicePlan {
    pub service: ServiceId,
    pub port: u16,
    pub closed: bool,
}
