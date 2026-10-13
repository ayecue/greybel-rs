//! Scheduled world work and its clock: routine definitions, queue, completion handling, mine jobs,
//! and routine implementations.

mod clock;
mod coin_round;
mod mine_jobs;
mod npc_switch;
mod routine;
mod schedule;
mod service_restart;
mod work;

pub use routine::{Routine, RoutineFailure, ScheduledRoutine, TickReport};
