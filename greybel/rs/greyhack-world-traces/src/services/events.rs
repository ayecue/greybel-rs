//! EventQueue published by administrator traces.

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
