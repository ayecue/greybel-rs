//! EventQueue published by guild traces.

use greyhack_world_modules::Event;

use crate::{ComputerId, TraceKind};

/// A trace caught the player and added a visible strike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrikeEvent {
    /// Computer on which the triggering action ran.
    pub computer: ComputerId,
    /// Active or passive trace that caught the player.
    pub trace: TraceKind,
    /// Visible strike count after this strike.
    pub strikes: u8,
}

impl Event for StrikeEvent {}

/// CrownShield clears floods targeting a computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloodClear {
    /// Computer whose flood subscribers are removed.
    pub computer: ComputerId,
}

impl Event for FloodClear {}
