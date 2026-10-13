#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Remote peer health.
pub enum Health {
    /// The remote peer responded to health checks.
    Healthy,
    /// More than `unhealthy_threshold` consecutive health checks were missed.
    Unhealthy,
}
