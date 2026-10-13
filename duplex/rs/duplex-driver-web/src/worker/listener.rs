use std::pin::Pin;
use std::task::{Context, Poll};

use futures_channel::mpsc;
use futures_core::Stream;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::MessageEvent;

use super::accept::Accept;
use super::scope::WorkerScope;
use crate::driver::MessageTarget;

const MESSAGE: &str = "message";

/// Stream returned by [`WorkerScope::listen`]. Yields [`WorkerScope`] for each valid host port-configuration message
/// in the [`WorkerScope::accept`] format.
pub struct WorkerListener {
    scope: MessageTarget,
    listener: Closure<dyn FnMut(MessageEvent)>,
    receiver: mpsc::UnboundedReceiver<WorkerScope>,
}

impl WorkerListener {
    pub(super) fn listen(scope: MessageTarget) -> Self {
        let (sender, receiver) = mpsc::unbounded();
        let listener = Closure::new(move |event: MessageEvent| {
            if let Some(scope) = Accept::scope(&event) {
                let _ = sender.unbounded_send(scope);
            }
        });
        let _ = scope.add_event_listener_with_callback(MESSAGE, listener.as_ref().unchecked_ref());

        Self {
            scope,
            listener,
            receiver,
        }
    }
}

impl Stream for WorkerListener {
    type Item = WorkerScope;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<WorkerScope>> {
        Pin::new(&mut self.receiver).poll_next(context)
    }
}

impl Drop for WorkerListener {
    fn drop(&mut self) {
        let _ = self
            .scope
            .remove_event_listener_with_callback(MESSAGE, self.listener.as_ref().unchecked_ref());
    }
}
