//! EventQueue published by guild services.

use greyhack_world_modules::Event;

use crate::ComputerId;

/// CrownShield clears floods targeting a computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloodClear {
    /// Computer whose flood subscribers are removed.
    pub computer: ComputerId,
}

impl Event for FloodClear {}
