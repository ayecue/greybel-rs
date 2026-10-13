use std::cell::{Cell, RefCell};
use std::mem;
use std::time::Duration;

use scheduler::Clock;

use crate::screen::Screen;
use crate::stdout::Stdout;
use crate::types::{Output, PrintOptions};

pub const DEFAULT_INTERVAL: Duration = Duration::from_millis(16);
pub const DEFAULT_CAPACITY: usize = 64 * 1024;

pub struct BufferedStdout<S: Screen, W: Clock> {
    screen: S,
    clock: W,
    queue: RefCell<Vec<Output>>,
    queued_bytes: Cell<usize>,
    last_batch: Cell<Option<Duration>>,
    interval: Duration,
    capacity: usize,
}

impl<S: Screen, W: Clock> BufferedStdout<S, W> {
    pub fn new(screen: S, clock: W) -> Self {
        Self {
            screen,
            clock,
            queue: RefCell::new(Vec::new()),
            queued_bytes: Cell::new(0),
            last_batch: Cell::new(None),
            interval: DEFAULT_INTERVAL,
            capacity: DEFAULT_CAPACITY,
        }
    }

    #[must_use]
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;

        self
    }

    #[must_use]
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;

        self
    }

    pub fn screen(&self) -> &S {
        &self.screen
    }

    fn push(&self, output: Output) {
        if let Output::Print { text, .. } = &output {
            self.queued_bytes.set(self.queued_bytes.get() + text.len());
        }

        self.queue.borrow_mut().push(output);
        if self.queued_bytes.get() >= self.capacity || self.is_due() {
            self.flush();
        }
    }

    fn is_due(&self) -> bool {
        self.last_batch
            .get()
            .is_none_or(|at| self.clock.now().saturating_sub(at) >= self.interval)
    }
}

impl<S: Screen + Default, W: Clock + Default> Default for BufferedStdout<S, W> {
    fn default() -> Self {
        Self::new(S::default(), W::default())
    }
}

impl<S: Screen, W: Clock> Stdout for BufferedStdout<S, W> {
    fn print(&self, text: &str, options: PrintOptions) {
        self.push(Output::Print {
            text: text.to_owned(),
            options,
        });
    }

    fn clear(&self) {
        self.queue.borrow_mut().clear();
        self.queued_bytes.set(0);
        self.push(Output::Clear);
    }

    fn flush(&self) {
        let batch = mem::take(&mut *self.queue.borrow_mut());

        if batch.is_empty() {
            return;
        }

        self.queued_bytes.set(0);
        self.screen.show(&batch);
        self.last_batch.set(Some(self.clock.now()));
    }

    fn flush_if_due(&self) {
        if self.is_due() {
            self.flush();
        }
    }
}

impl<S: Screen, W: Clock> Drop for BufferedStdout<S, W> {
    fn drop(&mut self) {
        self.flush();
    }
}
