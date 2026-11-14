use std::any::Any;
use std::rc::Rc;

use greyhack_world_routines::Entry;

use crate::{MineJobFailure, NpcEvent};

/// Routine and its due world second.
#[derive(Clone)]
pub struct ScheduledRoutine {
    /// Routine database row ID.
    pub id: u64,
    /// World second at which the routine is due.
    pub at: u64,
    /// Stable key of its routine kind.
    pub key: &'static str,
    data: Rc<dyn Any>,
}

impl ScheduledRoutine {
    /// Reads a scheduled routine from its queue entry.
    pub fn from_entry(entry: Entry) -> Self {
        Self {
            id: entry.id,
            at: entry.at,
            key: entry.key,
            data: Rc::from(entry.into_any()),
        }
    }

    /// Returns this routine's typed data.
    pub fn data<T: Any>(&self) -> Option<&T> {
        self.data.downcast_ref()
    }
}

impl PartialEq for ScheduledRoutine {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.at == other.at && self.key == other.key
    }
}

impl Eq for ScheduledRoutine {}

impl std::fmt::Debug for ScheduledRoutine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ScheduledRoutine")
            .field("id", &self.id)
            .field("at", &self.at)
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

/// EventQueue and errors produced while advancing the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickReport {
    /// World second at which the report started.
    pub at: u64,
    /// Formatted world date.
    pub date: String,
    /// Keys of executed routines in execution order.
    pub routines: Vec<&'static str>,
    /// NPC login changes produced by routines.
    pub npc_events: Vec<NpcEvent>,
    /// Routine failures that do not stop the tick.
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
