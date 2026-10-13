mod timer;

use std::time::Duration;

pub use self::timer::WebTimer;
use crate::{Clock, LocalTask, Scheduler, Sleep};

/// Scheduler using browser `performance.now`, `setTimeout`, `setInterval`, and microtask-based deferred callbacks
/// in a window or worker.
#[derive(Debug, Clone, Copy, Default)]
pub struct WebScheduler;

impl Clock for WebScheduler {
    fn now(&self) -> Duration {
        WebTimer::now()
    }

    fn sleep(&self, delay: Duration) -> Sleep {
        Sleep::after(self, delay)
    }
}

impl Scheduler for WebScheduler {
    type Timer = WebTimer;

    fn defer(&self, task: Box<dyn FnOnce()>) {
        wasm_bindgen_futures::spawn_local(async move { task() });
    }

    fn after(&self, delay: Duration, task: Box<dyn FnOnce()>) -> WebTimer {
        WebTimer::once(delay, task)
    }

    fn interval(&self, interval: Duration, task: Box<dyn Fn()>) -> WebTimer {
        WebTimer::interval(interval, task)
    }

    fn spawn(&self, task: LocalTask) {
        wasm_bindgen_futures::spawn_local(task);
    }
}
