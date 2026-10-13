use std::rc::Rc;
use std::time::Duration;

use crate::{LocalTask, Sleep};

/// Monotonic clock and asynchronous sleep interface.
pub trait Clock {
    /// Returns monotonically nondecreasing time since an implementation-defined epoch.
    fn now(&self) -> Duration;
    /// Returns a future that resolves after `delay`.
    fn sleep(&self, delay: Duration) -> Sleep;
}

/// Forwards clock operations to the shared instance.
impl<C: Clock + ?Sized> Clock for Rc<C> {
    fn now(&self) -> Duration {
        C::now(self)
    }

    fn sleep(&self, delay: Duration) -> Sleep {
        C::sleep(self, delay)
    }
}

/// A [`Clock`] that also schedules deferred callbacks and cancellable timers. Clones share scheduler state.
pub trait Scheduler: Clock + Clone + 'static {
    /// Cancels the associated one-shot or interval timer on drop.
    type Timer: 'static;

    /// Runs `task` after the current task yields.
    fn defer(&self, task: Box<dyn FnOnce()>);
    /// Runs `task` after `delay` unless the returned timer is dropped first.
    fn after(&self, delay: Duration, task: Box<dyn FnOnce()>) -> Self::Timer;
    /// Runs `task` at each `interval` until the returned timer is dropped.
    fn interval(&self, interval: Duration, task: Box<dyn Fn()>) -> Self::Timer;
    /// Runs future `task` to completion in a separate task on the current thread.
    fn spawn(&self, task: LocalTask);
}
