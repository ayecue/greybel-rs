//! Scheduled world work and its clock: queue access, completion handling, and mine jobs.

pub(crate) mod clock;
pub(crate) mod routine;
pub(crate) mod schedule;

pub use routine::{RoutineFailure, ScheduledRoutine, TickReport};
pub(crate) use schedule::RoutineService;
