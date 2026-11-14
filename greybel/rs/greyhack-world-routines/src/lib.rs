//! Scheduler of the Grey Hack world's timed work, independent of the world: no game rules.
//!
//! The queue stores a stable string key for each routine. A [`Routine`] instance owns its data
//! table and the services it calls. [`Scheduler`] resolves due rows to those instances.

mod kind;
mod scheduler;

pub use kind::{BoxFuture, Result, Routine, RoutineError, RoutineId};
pub use scheduler::{Entry, QUEUE_TABLES, Scheduler, Window};
