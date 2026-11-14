//! Traces of the player by network administrators.

use greyhack_behaviour::PassiveStage;

use crate::{ComputerId, PersonId};

/// Active trace row ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceId(pub(crate) u32);

/// An administrator tracing the player (`ActiveTraceSystem.ActiveTrace`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveTrace {
    pub id: TraceId,
    /// The computer whose network the administrator watches.
    pub computer: ComputerId,
    pub admin: PersonId,
    /// The program that caused the trace.
    pub terminal: u64,
    /// World second the trace ends the player's connection.
    pub ends_at: u64,
}

/// Passive trace progress (`PassiveTrace.ManageTraces`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PassiveState {
    pub running: bool,
    pub stage: PassiveStage,
    pub from_ip: String,
    pub to_ip: String,
    pub origin: Option<ComputerId>,
}
