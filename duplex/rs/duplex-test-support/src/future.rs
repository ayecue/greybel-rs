//! Future polling helpers for the current test thread.

use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::Thread;

/// Unparks the [`block_on`] thread when the future wakes.
struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

/// Polls `future` on the current thread and parks between wakes until completion.
pub fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut context = Context::from_waker(&waker);

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

/// Polls `future` once using a no-op waker.
pub fn poll<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    Pin::new(future).poll(&mut Context::from_waker(Waker::noop()))
}

/// Polls `future` once and returns its output, or panics if pending.
pub fn ready_output<F: Future>(future: F) -> F::Output {
    match poll(&mut pin!(future)) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("the future isn't ready"),
    }
}
