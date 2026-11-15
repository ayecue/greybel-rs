//! EventQueue published by guild services.

use greyhack_world_core::ComputerId;
use greyhack_world_effects::Event;

/// CrownShield clears floods targeting a computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloodClear {
    /// Computer whose flood subscribers are removed.
    pub computer: ComputerId,
}

impl Event for FloodClear {}
