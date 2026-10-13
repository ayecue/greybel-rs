use std::time::Duration;

use ::tokio::task::{self, JoinHandle};
use ::tokio::time::{self, MissedTickBehavior};

/// Tokio one-shot or interval task handle for the current `LocalSet`. Drop aborts the task.
pub struct TokioTimer(JoinHandle<()>);

impl TokioTimer {
    pub(super) fn interval(interval: Duration, task: Box<dyn Fn()>) -> Self {
        Self(task::spawn_local(async move {
            let mut ticks = time::interval_at(time::Instant::now() + interval, interval);

            ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                ticks.tick().await;
                task();
            }
        }))
    }

    pub(super) fn once(delay: Duration, task: Box<dyn FnOnce()>) -> Self {
        Self(task::spawn_local(async move {
            time::sleep(delay).await;
            task();
        }))
    }
}

impl Drop for TokioTimer {
    fn drop(&mut self) {
        self.0.abort();
    }
}
