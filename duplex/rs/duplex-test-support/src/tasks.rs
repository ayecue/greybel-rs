//! Shared futures polled with [`MemoryQueue`] batch delivery.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use crate::MemoryQueue;
use crate::future::poll;

type Task = Pin<Box<dyn Future<Output = ()>>>;

/// Cloneable queue for test- and handler-spawned futures.
#[derive(Clone, Default)]
pub struct TaskQueue(Rc<RefCell<Vec<Task>>>);

impl TaskQueue {
    pub fn spawn(&self, future: impl Future<Output = ()> + 'static) {
        self.0.borrow_mut().push(Box::pin(future));
    }

    /// Polls every pending future once and returns whether any completed.
    pub fn poll(&self) -> bool {
        let tasks = std::mem::take(&mut *self.0.borrow_mut());
        let count = tasks.len();
        let pending_tasks: Vec<Task> = tasks
            .into_iter()
            .filter_map(|mut task| poll(&mut task).is_pending().then_some(task))
            .collect();
        let any_ended = pending_tasks.len() < count;

        self.0.borrow_mut().extend(pending_tasks);

        any_ended
    }

    /// Alternates batch delivery and future polling until neither makes progress.
    pub fn run(&self, queue: &MemoryQueue) {
        loop {
            let step_count = queue.deliver();
            let any_ended = self.poll();

            if step_count == 0 && !any_ended {
                return;
            }
        }
    }
}
