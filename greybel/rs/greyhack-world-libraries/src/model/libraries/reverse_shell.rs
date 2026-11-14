//! Reverse-shell registrations owned by the libraries domain.

use greyhack_world_core::ComputerId;

/// Router reverse-shell registration (`RouterConfigOS`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReverseShell {
    pub computer: ComputerId,
    pub port: u16,
    pub user: String,
    pub pid: u32,
    /// Creation world second (`HelperReverseShell.createdAt`). Rebooting its computer after
    /// creation breaks the shell.
    pub creation_second: u64,
}
