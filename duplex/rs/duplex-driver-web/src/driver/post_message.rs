use std::cell::RefCell;
use std::hash::{BuildHasher, RandomState};

use duplex_core::{Driver, InboundBufferManager, OutboundBatch, Transport, WeakTransport};
use scheduler::{Clock, WebScheduler};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, MessagePort};

use super::target::MessageTarget;

const MESSAGE: &str = "message";

/// `duplex-core` driver exchanging transport batches via `postMessage` on a worker, global scope, or `MessagePort`.
pub struct PostMessage {
    target: MessageTarget,
    /// Generates non-cryptographic IDs for distinguishing transport connections.
    connection_id_rng: RefCell<fastrand::Rng>,
    listener: Closure<dyn FnMut(MessageEvent)>,
}

impl PostMessage {
    /// Creates a transport with channels communicating through `target`.
    #[must_use]
    pub fn transport(target: MessageTarget) -> Transport<Self> {
        Transport::new(|transport| Self::new(target, transport))
    }

    /// Creates a `target` transport secured by `security`.
    #[cfg(feature = "security")]
    #[must_use]
    pub fn transport_with_security(
        target: MessageTarget,
        security: duplex_core::Security,
    ) -> Transport<Self> {
        Transport::with_security(|transport| Self::new(target, transport), security)
    }

    fn new(target: MessageTarget, transport: WeakTransport<Self>) -> Self {
        let listener = Self::listener(transport);
        let _ = target.add_event_listener_with_callback(MESSAGE, listener.as_ref().unchecked_ref());

        if let Some(port) = target.dyn_ref::<MessagePort>() {
            port.start();
        }

        let seed = RandomState::new().hash_one(WebScheduler.now());

        Self {
            target,
            connection_id_rng: RefCell::new(fastrand::Rng::with_seed(seed)),
            listener,
        }
    }

    fn listener(transport: WeakTransport<Self>) -> Closure<dyn FnMut(MessageEvent)> {
        let mut buffers = InboundBufferManager::default();

        Closure::new(move |event: MessageEvent| {
            if let Some(payload) = Self::bytes(&event.data()) {
                let length = payload.length() as usize;

                transport.receive(buffers.batch(length, |batch| payload.copy_to(batch)));
            }
        })
    }

    fn bytes(data: &JsValue) -> Option<js_sys::Uint8Array> {
        if data.is_instance_of::<js_sys::Uint8Array>() {
            Some(data.clone().unchecked_into())
        } else if data.is_instance_of::<js_sys::ArrayBuffer>() {
            Some(js_sys::Uint8Array::new(data))
        } else {
            None
        }
    }
}

impl Driver for PostMessage {
    type Scheduler = WebScheduler;

    const CAPABILITIES: duplex_core::Capabilities = duplex_core::Capabilities::DUPLEX;

    fn transport_name(&self) -> &'static str {
        "worker"
    }

    fn send(&self, batch: OutboundBatch) {
        let batch = js_sys::Uint8Array::from(batch.as_slice());
        let transfer = js_sys::Array::of1(&batch.buffer());

        self.target.post_message_with_transfer(&batch, &transfer);
    }

    fn scheduler(&self) -> &WebScheduler {
        &WebScheduler
    }

    fn random(&self) -> [u8; 16] {
        let mut connection_id_rng = self.connection_id_rng.borrow_mut();

        std::array::from_fn(|_| connection_id_rng.u8(..))
    }
}

impl Drop for PostMessage {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(MESSAGE, self.listener.as_ref().unchecked_ref());
    }
}
