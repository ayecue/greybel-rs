use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use crate::Scheduler;

/// Future returned from [`crate::Clock::sleep`].
pub struct Sleep(Pin<Box<dyn Future<Output = ()>>>);

impl Sleep {
    /// Wraps `future` in a [`Sleep`].
    pub fn new(future: impl Future<Output = ()> + 'static) -> Self {
        Self(Box::pin(future))
    }

    /// Returns an immediately resolved sleep.
    #[must_use]
    pub fn ready() -> Self {
        Self::new(std::future::ready(()))
    }

    /// Returns a sleep using a one-shot `scheduler` timer. Dropping the sleep cancels the timer.
    pub fn after<S: Scheduler>(scheduler: &S, delay: Duration) -> Self {
        Self::until(|fire| scheduler.after(delay, fire))
    }

    /// Creates a sleep and passes its completion callback to `start`. Dropping the sleep also drops the timer
    /// returned by `start`.
    pub(crate) fn until<T: 'static>(start: impl FnOnce(Box<dyn FnOnce()>) -> T) -> Self {
        let wake = Rc::new(Wake::default());
        let timer = start({
            let wake = Rc::clone(&wake);

            Box::new(move || wake.fire())
        });

        Self::new(async move {
            std::future::poll_fn(|context| wake.poll(context)).await;

            drop(timer);
        })
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        self.0.as_mut().poll(context)
    }
}

/// Stores timer completion state and the associated sleep future's waker.
#[derive(Default)]
struct Wake {
    fired: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl Wake {
    fn fire(&self) {
        self.fired.set(true);
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
    }

    fn poll(&self, context: &Context<'_>) -> Poll<()> {
        if self.fired.get() {
            return Poll::Ready(());
        }

        self.waker.replace(Some(context.waker().clone()));

        Poll::Pending
    }
}
