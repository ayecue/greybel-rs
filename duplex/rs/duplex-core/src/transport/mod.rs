mod aliases;
mod dispatch;
mod endpoints;
mod inbound;
mod outbox;
mod relay;
mod security;
mod state;
mod work_queue;

use std::rc::{Rc, Weak};

pub(crate) use inbound::{Endpoint, Inbound};
pub use relay::{Port, PortReceiver, PortSender, Relay};
pub(crate) use state::TransportState;

use crate::definition::ErrorCode;
use crate::driver::{Driver, InboundBatch};

/// Multiplexes channels over one driver connection.
///
/// Batches outbound channel data for the driver and routes inbound batches to channels. The thread
/// [`Hub`](crate::Hub) processes received data.
pub struct Transport<D: Driver>(Rc<TransportState<D>>);

/// Weak transport reference for drivers.
pub struct WeakTransport<D: Driver>(Weak<TransportState<D>>);

impl<D: Driver> Transport<D> {
    /// Creates a transport and passes its weak reference to the driver constructor.
    pub fn new(driver: impl FnOnce(WeakTransport<D>) -> D) -> Self {
        let state = Rc::new_cyclic(|this| {
            TransportState::new(Weak::clone(this), driver(WeakTransport(Weak::clone(this))))
        });

        state.register();

        Self(state)
    }

    /// Returns the transport driver.
    pub fn driver(&self) -> &D {
        &self.0.driver
    }

    #[must_use]
    /// Returns a weak transport reference.
    pub fn downgrade(&self) -> WeakTransport<D> {
        WeakTransport(Rc::downgrade(&self.0))
    }

    /// Closes every channel with `reason` and does not notify the remote peer.
    pub fn close_all(&self, reason: ErrorCode) {
        for endpoint in self.0.endpoints() {
            endpoint.close(reason);
        }

        self.0.flush_all();
    }

    pub(crate) fn state(&self) -> &Rc<TransportState<D>> {
        &self.0
    }
}

impl<D: Driver> Clone for Transport<D> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<D: Driver> WeakTransport<D> {
    #[must_use]
    /// Returns the transport, or `None` after it is dropped.
    pub fn upgrade(&self) -> Option<Transport<D>> {
        self.0.upgrade().map(Transport)
    }

    /// Routes an inbound batch to channels.
    pub fn receive(&self, batch: impl Into<InboundBatch>) {
        if let Some(transport) = self.0.upgrade() {
            transport.receive(batch.into());
        }
    }

    /// Permanently closes the transport and all channels with `ConnectionLoss`.
    pub fn on_disconnect(&self) {
        if let Some(transport) = self.0.upgrade() {
            transport.on_disconnect();
        }
    }
}

impl<D: Driver> Clone for WeakTransport<D> {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}
