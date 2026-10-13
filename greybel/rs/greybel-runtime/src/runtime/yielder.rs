use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use super::RuntimeState;
use super::host::Host;

/// Callback whose future resolves after the host handles queued events, including incoming worker messages.
pub type Yielder = Rc<dyn Fn() -> Pin<Box<dyn Future<Output = ()>>>>;

impl<H: Host> RuntimeState<H> {
    /// Polls `future` and invokes the host yielder when it self-wakes while pending, allowing browser hosts to handle
    /// editor messages between microtasks.
    pub(crate) async fn run_with_yields<F: Future>(&self, future: F) -> F::Output {
        let Some(yielder) = self.yielder.borrow().clone() else {
            return future.await;
        };
        let mut future = pin!(future);
        let signal = Arc::new(Signal::default());
        let mut pending_yield: Option<Pin<Box<dyn Future<Output = ()>>>> = None;

        poll_fn(|context| {
            if let Some(wait) = &mut pending_yield {
                if wait.as_mut().poll(context).is_pending() {
                    return Poll::Pending;
                }

                pending_yield = None;
            }

            signal.watch(context.waker());

            let waker = Waker::from(Arc::clone(&signal));
            let poll_result = future.as_mut().poll(&mut Context::from_waker(&waker));
            let woke_itself = signal.unwatch();

            if poll_result.is_pending() && woke_itself {
                let mut wait = yielder();

                if wait.as_mut().poll(context).is_ready() {
                    context.waker().wake_by_ref();
                } else {
                    pending_yield = Some(wait);
                }
            }

            poll_result
        })
        .await
    }
}

/// Wraps the inner future's waker: wakes during polling mark self-yield; other wakes notify the parent task.
#[derive(Default)]
struct Signal {
    task: Mutex<Option<Waker>>,
    polling: AtomicBool,
    woken: AtomicBool,
}

impl Signal {
    fn watch(&self, task: &Waker) {
        let mut slot = self
            .task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if !slot.as_ref().is_some_and(|waker| waker.will_wake(task)) {
            *slot = Some(task.clone());
        }

        self.polling.store(true, Ordering::Relaxed);
    }

    fn unwatch(&self) -> bool {
        self.polling.store(false, Ordering::Relaxed);

        self.woken.swap(false, Ordering::Relaxed)
    }
}

impl Wake for Signal {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if self.polling.load(Ordering::Relaxed) {
            self.woken.store(true, Ordering::Relaxed);
        } else if let Some(task) = self
            .task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            task.wake_by_ref();
        }
    }
}
