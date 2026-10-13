use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_channel::oneshot;

use crate::definition::ErrorCode;

/// Future resolving to the channel close reason.
#[must_use = "futures do nothing unless awaited"]
pub struct CloseFuture(State);

enum State {
    Ready(ErrorCode),
    Pending(oneshot::Receiver<ErrorCode>),
}

impl CloseFuture {
    pub(crate) fn ready(reason: ErrorCode) -> Self {
        Self(State::Ready(reason))
    }

    pub(crate) fn from_receiver(reason: oneshot::Receiver<ErrorCode>) -> Self {
        Self(State::Pending(reason))
    }
}

impl Future for CloseFuture {
    type Output = ErrorCode;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<ErrorCode> {
        match &mut self.get_mut().0 {
            State::Ready(reason) => Poll::Ready(*reason),
            State::Pending(reason) => Pin::new(reason)
                .poll(context)
                .map(|reason| reason.unwrap_or(ErrorCode::ClosedChannel)),
        }
    }
}
