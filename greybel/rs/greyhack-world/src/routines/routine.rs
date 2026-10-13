use greyhack_behaviour::ServiceId;

use crate::{CoinId, ComputerId, MineJobFailure, NpcEvent, PersonId, TraceId};

/// World work scheduled for a specific world time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routine {
    ServiceRestart {
        computer: ComputerId,
        service: ServiceId,
    },
    /// Coin mining round that rewards one mine job. Coins with jobs have one round per mining
    /// cycle.
    CoinRound { coin: CoinId },
    /// NPC login or logout.
    NpcSwitch { npc: PersonId },
    /// An active trace runs out (`ActiveTraceSystem.ProgressActiveTrace`).
    ActiveTrace { trace: TraceId },
    /// The passive trace takes its next step (`PassiveTrace.ManageTraces`).
    PassiveTrace,
}

/// Routine and its due world second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledRoutine {
    /// Routine database row ID.
    pub id: u64,
    pub at: u64,
    pub routine: Routine,
}

/// Events and errors produced while advancing the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickReport {
    pub at: u64,
    pub date: String,
    /// Executed routines in execution order.
    pub routines: Vec<Routine>,
    pub npc_events: Vec<NpcEvent>,
    pub errors: Vec<RoutineFailure>,
}

/// Routine execution failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutineFailure {
    /// A mine job in a coin round failed.
    MineJob(MineJobFailure),
    /// The service-restart computer no longer exists.
    ServiceComputerGone,
}
