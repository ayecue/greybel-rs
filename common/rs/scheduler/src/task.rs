//! Runs futures to completion on the thread of a scheduler that only supports deferred callbacks.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Wake, Waker};
use std::thread::{self, ThreadId};

use crate::Scheduler;

/// A future run to completion by a scheduler on its thread.
pub type LocalTask = Pin<Box<dyn Future<Output = ()>>>;

/// Defers a scheduler callback.
type Defer = Box<dyn Fn(Box<dyn FnOnce()>)>;

/// Polls `task` through `scheduler` deferred callbacks initially and after each wake until completion. Use this for
/// schedulers without an executor, such as test schedulers.
///
/// # Panics
///
/// The task waker panics if called from a thread other than its spawning thread.
pub fn spawn_through_defer<S: Scheduler>(scheduler: &S, task: LocalTask) {
    let scheduler = scheduler.clone();
    let id = NEXT_ID.with(|next| next.replace(next.get() + 1));
    let task = Rc::new(DeferredTask {
        future: RefCell::new(Some(task)),
        queued: Cell::new(false),
        defer: Box::new(move |poll| scheduler.defer(poll)),
        waker: Waker::from(Arc::new(TaskWaker {
            thread: thread::current().id(),
            id,
        })),
    });

    TASKS.with(|tasks| tasks.borrow_mut().insert(id, Rc::clone(&task)));
    task.queue(id);
}

thread_local! {
    static TASKS: RefCell<HashMap<u64, Rc<DeferredTask>>> = RefCell::new(HashMap::new());
    static NEXT_ID: Cell<u64> = const { Cell::new(0) };
}

/// State for [`spawn_through_defer`]. The future is absent while polled and after completion.
struct DeferredTask {
    future: RefCell<Option<LocalTask>>,
    /// Whether a poll is already deferred.
    queued: Cell<bool>,
    defer: Defer,
    waker: Waker,
}

impl DeferredTask {
    /// Defers a poll for task `id` if none is pending.
    fn queue(self: &Rc<Self>, id: u64) {
        if self.queued.replace(true) {
            return;
        }

        let task = Rc::clone(self);

        (self.defer)(Box::new(move || task.poll(id)));
    }

    fn poll(&self, id: u64) {
        self.queued.set(false);

        let Some(mut future) = self.future.borrow_mut().take() else {
            return;
        };

        if future
            .as_mut()
            .poll(&mut Context::from_waker(&self.waker))
            .is_ready()
        {
            TASKS.with(|tasks| tasks.borrow_mut().remove(&id));
        } else {
            self.future.replace(Some(future));
        }
    }
}

/// Wakes task `id` belonging to thread `thread`.
struct TaskWaker {
    thread: ThreadId,
    id: u64,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        assert_eq!(
            thread::current().id(),
            self.thread,
            "a task of a deferring scheduler is woken on its own thread only"
        );

        let task = TASKS.with(|tasks| tasks.borrow().get(&self.id).cloned());

        if let Some(task) = task {
            task.queue(self.id);
        }
    }
}
