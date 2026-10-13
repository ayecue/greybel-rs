mod tasks;
mod timer;

use std::rc::Rc;
use std::time::Duration;

use ::tokio::time::Instant;

use self::tasks::TaskQueue;
pub use self::timer::TokioTimer;
use crate::{Clock, LocalTask, Scheduler, Sleep};

/// Scheduler using Tokio's clock and timers. A task in the current `LocalSet` runs deferred callbacks sequentially
/// and starts when the first callback is queued. Clones share the task queue and clock origin.
#[derive(Clone)]
pub struct TokioScheduler(Rc<State>);

struct State {
    tasks: TaskQueue,
    start: Instant,
}

impl TokioScheduler {
    /// Creates a scheduler whose clock origin is the current Tokio instant. Tasks and timers require a current
    /// `LocalSet` and a time-enabled Tokio runtime.
    #[must_use]
    pub fn new() -> Self {
        Self(Rc::new(State {
            tasks: TaskQueue::default(),
            start: Instant::now(),
        }))
    }
}

impl Default for TokioScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for TokioScheduler {
    fn now(&self) -> Duration {
        self.0.start.elapsed()
    }

    fn sleep(&self, delay: Duration) -> Sleep {
        Sleep::new(::tokio::time::sleep(delay))
    }
}

impl Scheduler for TokioScheduler {
    type Timer = TokioTimer;

    /// # Panics
    ///
    /// Panics outside a `LocalSet` if the runner task has not started.
    fn defer(&self, task: Box<dyn FnOnce()>) {
        self.0.tasks.defer(task);
    }

    fn after(&self, delay: Duration, task: Box<dyn FnOnce()>) -> TokioTimer {
        TokioTimer::once(delay, task)
    }

    fn interval(&self, interval: Duration, task: Box<dyn Fn()>) -> TokioTimer {
        TokioTimer::interval(interval, task)
    }

    /// # Panics
    ///
    /// Panics when called outside a `LocalSet`.
    fn spawn(&self, task: LocalTask) {
        ::tokio::task::spawn_local(task);
    }
}
