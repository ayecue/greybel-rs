use greyhack_behaviour::ServiceId;

/// Planner service with probability and closed state (`Servicio`, `probabilidad`, `isClosed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PoolService {
    pub service: ServiceId,
    pub chance: u8,
    pub closed: bool,
}
