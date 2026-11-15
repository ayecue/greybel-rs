//! Core domain events.

use greyhack_world_effects::Event;

use crate::ComputerId;

/// A world tick or bounded step is about to advance the clock and run routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockAdvance;

impl Event for ClockAdvance {}

/// A world tick or bounded step has advanced the clock and run its routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockAdvanceEnd;

impl Event for ClockAdvanceEnd {}

/// A computer rebooted and its connections closed (`HelperScript.CheckBreakConnecion`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebootEvent {
    /// RebootEvent computer.
    pub computer: ComputerId,
}

impl Event for RebootEvent {}
