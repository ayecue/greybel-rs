use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(super) struct JoinAll<F: Future> {
    futures: Vec<Pin<Box<F>>>,
    outputs: Vec<Option<F::Output>>,
}

impl<F: Future> JoinAll<F> {
    pub(super) fn new(futures: impl IntoIterator<Item = F>) -> Self {
        let futures: Vec<_> = futures.into_iter().map(Box::pin).collect();
        let outputs = futures.iter().map(|_| None).collect();

        Self { futures, outputs }
    }
}

impl<F: Future> Unpin for JoinAll<F> {}

impl<F: Future> Future for JoinAll<F> {
    type Output = Vec<F::Output>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        let mut pending = false;

        for (future, output) in this.futures.iter_mut().zip(&mut this.outputs) {
            if output.is_none() {
                match future.as_mut().poll(context) {
                    Poll::Ready(value) => *output = Some(value),
                    Poll::Pending => pending = true,
                }
            }
        }

        if pending {
            return Poll::Pending;
        }

        Poll::Ready(
            this.outputs
                .drain(..)
                .map(|output| output.expect("every future is done"))
                .collect(),
        )
    }
}
