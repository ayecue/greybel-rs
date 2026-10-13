use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use futures_channel::oneshot;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, MessagePort};

use super::scope::WorkerScope;
use crate::driver::{MessageTarget, PostMessage};

const MESSAGE: &str = "message";

/// Future returned by [`WorkerScope::accept`]. Resolves after a valid initialization message with a `Uint32Array`
/// of channel IDs and `MessagePort`s. The first port is the worker transport; each remaining port relays the channel
/// at its array index.
pub(super) struct Accept {
    scope: MessageTarget,
    listener: Closure<dyn FnMut(MessageEvent)>,
    receiver: oneshot::Receiver<WorkerScope>,
}

impl Accept {
    /// Listens for initialization messages on `scope` and returns the pending accept future.
    pub(super) fn listen(scope: MessageTarget) -> Self {
        let (sender, receiver) = oneshot::channel();
        let sender = RefCell::new(Some(sender));
        let listener = Closure::new(move |event: MessageEvent| {
            let Some(scope) = Self::scope(&event) else {
                return;
            };

            if let Some(sender) = sender.borrow_mut().take() {
                let _ = sender.send(scope);
            }
        });
        let _ = scope.add_event_listener_with_callback(MESSAGE, listener.as_ref().unchecked_ref());

        Self {
            scope,
            listener,
            receiver,
        }
    }

    pub(super) fn scope(event: &MessageEvent) -> Option<WorkerScope> {
        let channels = event.data().dyn_into::<js_sys::Uint32Array>().ok()?;
        let ports: Vec<MessagePort> = event
            .ports()
            .iter()
            .map(JsCast::dyn_into)
            .collect::<Result<_, _>>()
            .ok()?;
        let (own, relay_ports) = ports.split_first()?;

        if relay_ports.len() != channels.length() as usize {
            return None;
        }

        let relays = channels
            .to_vec()
            .into_iter()
            .zip(relay_ports)
            .map(|(channel, port)| {
                (
                    channel,
                    PostMessage::transport(port.clone().unchecked_into()),
                )
            })
            .collect::<Rc<[_]>>();

        Some(WorkerScope::new(
            PostMessage::transport(own.clone().unchecked_into()),
            relays,
        ))
    }
}

impl Future for Accept {
    type Output = WorkerScope;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<WorkerScope> {
        match Pin::new(&mut self.receiver).poll(context) {
            Poll::Ready(Ok(scope)) => Poll::Ready(scope),

            Poll::Ready(Err(_)) | Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for Accept {
    fn drop(&mut self) {
        let _ = self
            .scope
            .remove_event_listener_with_callback(MESSAGE, self.listener.as_ref().unchecked_ref());
    }
}
