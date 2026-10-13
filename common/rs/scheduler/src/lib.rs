//! Clock and task-scheduling interfaces shared by project crates. [`Clock`] provides monotonic time and
//! asynchronous sleeps. [`Scheduler`] adds deferred callbacks, one-shot and interval timers, and independent tasks
//! for event-loop drivers. Implementations include [`TokioScheduler`] (feature `tokio`), [`WebScheduler`]
//! (feature `web`), and [`ManualClock`] (feature `manual`, for tests).

#![warn(missing_docs)]
mod clock;
#[cfg(feature = "manual")]
mod manual;
mod sleep;
mod task;
#[cfg(feature = "tokio")]
mod tokio;
#[cfg(feature = "web")]
mod web;

pub use clock::{Clock, Scheduler};
#[cfg(feature = "manual")]
pub use manual::{ManualClock, ManualTimer};
pub use sleep::Sleep;
pub use task::{LocalTask, spawn_through_defer};
#[cfg(feature = "tokio")]
pub use tokio::{TokioScheduler, TokioTimer};
#[cfg(feature = "web")]
pub use web::{WebScheduler, WebTimer};
