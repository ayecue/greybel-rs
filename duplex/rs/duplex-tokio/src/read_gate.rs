use std::cell::Cell;
use std::rc::Rc;

use tokio::sync::Notify;

/// Shared transport backpressure gate that pauses read loops under incoming overload.
#[derive(Clone, Default)]
pub struct ReadGate(Rc<Gate>);

#[derive(Default)]
struct Gate {
    closed: Cell<bool>,
    open_signal: Notify,
}

impl ReadGate {
    /// Pauses reads until [`resume`](Self::resume).
    pub fn pause(&self) {
        self.0.closed.set(true);
    }

    /// Resumes reads and wakes every waiting read loop.
    pub fn resume(&self) {
        self.0.closed.set(false);
        self.0.open_signal.notify_waiters();
    }

    /// Waits until the gate resumes.
    pub async fn wait(&self) {
        while self.0.closed.get() {
            self.0.open_signal.notified().await;
        }
    }
}
