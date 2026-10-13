use std::cell::{Cell, OnceCell, RefCell};
use std::collections::VecDeque;
use std::future::poll_fn;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;
use std::task::{Poll, Waker};

use ::tokio::task::{self, JoinHandle};

type Task = Box<dyn FnOnce()>;

/// FIFO deferred-callback queue drained sequentially by a `LocalSet` task. The first callback starts the runner;
/// dropping the queue aborts it.
#[derive(Default)]
pub(super) struct TaskQueue {
    queue: Rc<Queue>,
    runner: OnceCell<JoinHandle<()>>,
}

/// Pending callbacks and the waker that notifies the runner when the queue becomes non-empty.
#[derive(Default)]
struct Queue {
    tasks: RefCell<VecDeque<Task>>,
    runner_waker: Cell<Option<Waker>>,
}

impl TaskQueue {
    pub(super) fn defer(&self, task: Task) {
        self.runner
            .get_or_init(|| task::spawn_local(Self::run(Rc::clone(&self.queue))));
        self.queue.tasks.borrow_mut().push_back(task);
        if let Some(waker) = self.queue.runner_waker.take() {
            waker.wake();
        }
    }

    async fn run(queue: Rc<Queue>) {
        let mut ready_tasks = VecDeque::new();

        loop {
            poll_fn(|context| {
                if queue.tasks.borrow().is_empty() {
                    queue.runner_waker.set(Some(context.waker().clone()));

                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            })
            .await;
            std::mem::swap(&mut ready_tasks, &mut *queue.tasks.borrow_mut());
            for task in ready_tasks.drain(..) {
                let _ = panic::catch_unwind(AssertUnwindSafe(task));
            }

            if !queue.tasks.borrow().is_empty() {
                task::yield_now().await;
            }
        }
    }
}

impl Drop for TaskQueue {
    fn drop(&mut self) {
        if let Some(runner) = self.runner.get() {
            runner.abort();
        }
    }
}
