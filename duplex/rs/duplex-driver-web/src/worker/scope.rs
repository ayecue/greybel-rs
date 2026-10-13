use std::cell::RefCell;
use std::future::Future;
use std::panic;
use std::rc::Rc;

use duplex_core::{Channel, ChannelOptions, ErrorCode, Peer, Protocol, Transport, WeakTransport};
use wasm_bindgen::JsCast;

use super::accept::Accept;
use super::listener::WorkerListener;
use crate::driver::{MessageTarget, PostMessage};

thread_local! {
    /// Thread-local worker-scope transport registry used by the panic hook to close channels.
    static TRANSPORTS: RefCell<Vec<WeakTransport<PostMessage>>> = const { RefCell::new(Vec::new()) };
}

/// Worker primary transport and optional host-provided per-channel relay transports.
#[derive(Clone)]
pub struct WorkerScope {
    transport: Transport<PostMessage>,
    relays: Rc<[(u32, Transport<PostMessage>)]>,
}

impl WorkerScope {
    /// Listens for the host's initial port message and returns a future resolving to the worker scope. Call during
    /// module initialization to install the listener before the first host message arrives.
    pub fn accept() -> impl Future<Output = Self> {
        Accept::listen(js_sys::global().unchecked_into::<MessageTarget>())
    }

    /// Returns a stream of worker scopes for valid host port messages. Listening starts immediately as in
    /// [`WorkerScope::accept`]. Later messages can provide ports opened for other workers, such as through
    /// `WorkerHost.open` in `duplex-driver-worker`.
    #[must_use]
    pub fn listen() -> WorkerListener {
        WorkerListener::listen(js_sys::global().unchecked_into::<MessageTarget>())
    }

    pub(super) fn new(
        transport: Transport<PostMessage>,
        relays: Rc<[(u32, Transport<PostMessage>)]>,
    ) -> Self {
        TRANSPORTS.with(|transports| {
            let mut transports = transports.borrow_mut();

            transports.push(transport.downgrade());
            transports.extend(relays.iter().map(|(_, relay)| relay.downgrade()));
        });

        Self { transport, relays }
    }

    /// Returns the transport on the worker port.
    #[must_use]
    pub fn transport(&self) -> &Transport<PostMessage> {
        &self.transport
    }

    /// Returns the relay transport for protocol `P` channel, or `None` if the host provided none.
    #[must_use]
    pub fn port_transport_of<P: Protocol>(&self) -> Option<&Transport<PostMessage>> {
        self.relays
            .iter()
            .find(|(channel, _)| *channel == P::CHANNEL_ID)
            .map(|(_, relay)| relay)
    }

    /// Returns the relay transport for protocol `P` channel, or the worker transport if none is assigned.
    #[must_use]
    pub fn transport_of<P: Protocol>(&self) -> &Transport<PostMessage> {
        self.relays
            .iter()
            .find(|(channel, _)| *channel == P::CHANNEL_ID)
            .map_or(&self.transport, |(_, relay)| relay)
    }

    /// # Panics
    ///
    /// Panics if this scope already has a `P` channel.
    #[must_use]
    pub fn channel<P: Peer<OpenPayload = ()>>(&self) -> Channel<P, PostMessage> {
        self.channel_with_options(ChannelOptions::default())
    }

    /// # Panics
    ///
    /// Panics if this scope already has a `P` channel.
    #[must_use]
    pub fn channel_with_options<P: Peer<OpenPayload = ()>>(
        &self,
        options: ChannelOptions,
    ) -> Channel<P, PostMessage> {
        Channel::new(self.transport_of::<P::Protocol>(), options)
    }

    /// Installs a panic hook that calls the previous hook, logs to the browser console, and closes every channel on
    /// registered worker transports with `ErrorCode::Panic`.
    pub fn report_panics() {
        let previous = panic::take_hook();

        panic::set_hook(Box::new(move |info| {
            previous(info);
            web_sys::console::error_1(&info.to_string().into());

            let _ = TRANSPORTS.try_with(|transports| {
                let Ok(transports) = transports.try_borrow() else {
                    return;
                };

                for transport in transports.iter().filter_map(WeakTransport::upgrade) {
                    transport.close_all(ErrorCode::Panic);
                }
            });
        }));
    }
}
