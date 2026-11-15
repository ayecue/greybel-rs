#![deny(clippy::await_holding_refcell_ref)]

//! Effects of a Grey Hack world over time: ordered event dispatch, typed work awaited across
//! cycles, and persistent routines scheduled for later cycles.

mod events;
mod routines;
mod work;

pub use events::{Event, EventError, EventFuture, EventQueue};
pub use routines::{
    BoxFuture, Entry, QUEUE_TABLES, Result, Routine, RoutineError, RoutineId, Scheduler, Window,
};
pub use work::{
    Work, WorkCancel, WorkCancelFuture, WorkCompletion, WorkEnvelope, WorkId, WorkOwner, WorkQueue,
};
