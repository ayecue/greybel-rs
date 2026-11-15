//! Persistent scheduling of routines due in the current or a later world cycle.

mod kind;
mod scheduler;

pub use kind::{BoxFuture, Result, Routine, RoutineError, RoutineId};
pub use scheduler::{Entry, QUEUE_TABLES, Scheduler, Window};
