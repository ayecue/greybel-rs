//! Manual scheduler with on-demand deferred tasks and clock-driven timers.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use scheduler::{Clock, LocalTask, ManualClock, ManualTimer, Scheduler, Sleep};

type Task = Box<dyn FnOnce()>;

/// Shared manual scheduler state: deferred task queue, recorded but unexecuted interval requests, and a manually
/// advanced clock. Clock advancement runs one-shot timers.
#[derive(Clone, Default)]
pub struct ManualScheduler(Rc<State>);

#[derive(Default)]
struct State {
    tasks: RefCell<VecDeque<Task>>,
    intervals: RefCell<Vec<Duration>>,
    clock: ManualClock,
}

impl ManualScheduler {
    /// Runs the oldest deferred task and returns `true`, or `false` if the queue is empty.
    pub fn run_task(&self) -> bool {
        let task = self.0.tasks.borrow_mut().pop_front();
        let Some(task) = task else {
            return false;
        };

        task();

        true
    }

    /// Runs a FIFO snapshot of queued tasks; tasks deferred during execution stay queued.
    pub fn run_tasks_once(&self) {
        let tasks = std::mem::take(&mut *self.0.tasks.borrow_mut());

        for task in tasks {
            task();
        }
    }

    /// Advances the clock by `by` and runs newly due one-shot timers.
    pub fn advance(&self, by: Duration) {
        self.0.clock.advance(by);
    }

    /// Returns timer intervals in request registration order.
    #[must_use]
    pub fn intervals(&self) -> Vec<Duration> {
        self.0.intervals.borrow().clone()
    }
}

impl Clock for ManualScheduler {
    fn now(&self) -> Duration {
        self.0.clock.now()
    }

    fn sleep(&self, delay: Duration) -> Sleep {
        Sleep::after(self, delay)
    }
}

impl Scheduler for ManualScheduler {
    type Timer = ManualTimer;

    fn defer(&self, task: Task) {
        self.0.tasks.borrow_mut().push_back(task);
    }

    fn after(&self, delay: Duration, task: Task) -> ManualTimer {
        self.0.clock.after(delay, task)
    }

    fn interval(&self, interval: Duration, _task: Box<dyn Fn()>) -> ManualTimer {
        self.0.intervals.borrow_mut().push(interval);

        ManualClock::idle()
    }

    /// Queues `task` for polling through deferred-task runs after each wake.
    fn spawn(&self, task: LocalTask) {
        scheduler::spawn_through_defer(self, task);
    }
}
