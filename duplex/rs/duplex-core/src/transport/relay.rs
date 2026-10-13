use std::cell::{Cell, RefCell};
use std::future::{Future, poll_fn};
use std::ops::Range;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll};

use futures_channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures_core::Stream;

use super::aliases::RemoteAliasTable;
use super::{Endpoint, Inbound, Transport, TransportState};
use crate::binary::{BinaryReader, BinaryWriter};
use crate::channel::Bundle;
use crate::connection_id::ConnectionId;
use crate::definition::{ErrorCode, FrameKind, Role};
use crate::driver::Driver;
use crate::frame::{
    BatchReader, BatchWriter, CORE_MAJOR, Control, FrameEncoder, FrameReader, Route, SliceKey,
};
use crate::keys::EndpointId;
use crate::protocol::{Peer, Protocol};

const ONE_SLICE: u32 = 1;

/// Batch-channel endpoint connected to a spawned channel.
pub struct Port {
    sender: UnboundedSender<Vec<u8>>,
    receiver: UnboundedReceiver<Vec<u8>>,
}

/// Sending half of a spawned channel port.
pub struct PortSender(UnboundedSender<Vec<u8>>);

/// Receiving half of a spawned channel port.
pub struct PortReceiver(UnboundedReceiver<Vec<u8>>);

impl Port {
    fn pair() -> (Self, Self) {
        let (to_task, from_hub) = mpsc::unbounded();
        let (to_hub, from_task) = mpsc::unbounded();

        (
            Self {
                sender: to_task,
                receiver: from_task,
            },
            Self {
                sender: to_hub,
                receiver: from_hub,
            },
        )
    }

    #[must_use]
    /// Splits the port into sender and receiver.
    pub fn split(self) -> (PortSender, PortReceiver) {
        (PortSender(self.sender), PortReceiver(self.receiver))
    }
}

impl PortSender {
    /// Sends a batch or returns `false` if the channel is closed.
    #[must_use]
    pub fn send(&self, batch: &[u8]) -> bool {
        self.0.unbounded_send(batch.to_vec()).is_ok()
    }
}

impl PortReceiver {
    /// Returns the next received batch, or `None` after channel closure.
    pub async fn receive(&mut self) -> Option<Vec<u8>> {
        poll_fn(|context| Pin::new(&mut self.0).poll_next(context)).await
    }
}

#[must_use = "the spawned channel's batches only reach the transport while the relay is polled"]
/// Relays batches between a transport and a channel on another task or thread.
///
/// Dropping the spawned channel port closes the current transport-peer connection with `ConnectionLoss`. Transport
/// channel closure closes both sides; transport disconnection closes the spawned channel.
pub struct Relay<D: Driver> {
    link: Rc<RelayLink<D>>,
    handle: EndpointId,
    receiver: UnboundedReceiver<Vec<u8>>,
    aliases: RemoteAliasTable,
}

/// Transport endpoint forwarding inbound transport-peer batches to the spawned channel.
struct Forward<D: Driver> {
    to: RefCell<Option<UnboundedSender<Vec<u8>>>>,
    batch: RefCell<BinaryWriter>,
    link: Rc<RelayLink<D>>,
}

/// State shared by both relay directions.
///
/// Contains the channel transport role, spawned-channel batch bundle, and current connection. Slice routes and `Ack`
/// frames update the connection; a `Close` frame clears it.
struct RelayLink<D: Driver> {
    transport: Weak<TransportState<D>>,
    channel: u32,
    role: Role,
    remote: Role,
    bundle: Cell<Bundle>,
    connection: Cell<Option<ConnectionId>>,
}

impl<D: Driver> Transport<D> {
    /// Spawns a `P` channel and passes its port to `spawn`.
    ///
    /// # Panics
    ///
    /// Panics if the transport already has a `P` protocol channel.
    pub fn spawn<P: Peer>(&self, spawn: impl FnOnce(Port)) -> Relay<D> {
        let (here, there) = Port::pair();
        let link = Rc::new(RelayLink {
            transport: self.0.weak(),
            channel: <P::Protocol as Protocol>::CHANNEL_ID,
            role: P::ROLE,
            remote: <P::Remote as Peer>::ROLE,
            bundle: Cell::new(Bundle::Common),
            connection: Cell::new(None),
        });
        let mut handle = None;

        self.0.attach(link.channel, P::ROLE, |id| {
            handle = Some(id);

            Rc::new(Forward {
                to: RefCell::new(Some(here.sender)),
                batch: RefCell::default(),
                link: Rc::clone(&link),
            })
        });
        spawn(there);

        Relay {
            link,
            handle: handle.expect("the channel was just attached"),
            receiver: here.receiver,
            aliases: RemoteAliasTable::default(),
        }
    }
}

impl<D: Driver> Relay<D> {
    /// Sets the batch bundle for the spawned channel.
    pub fn set_bundle(self, bundle: Bundle) -> Self {
        self.link.bundle.set(bundle);

        self
    }

    fn forward(&mut self, transport: &TransportState<D>, batch: &[u8]) {
        let Ok(slices) = BatchReader::new(batch) else {
            return;
        };

        if !slices.is_of_this_core() {
            return;
        }

        let link = &self.link;

        for slice in slices {
            let Ok(slice) = slice else {
                return;
            };
            let route = self.aliases.route(slice.key);
            let Some(route) =
                route.filter(|route| route.channel == link.channel && route.from == link.role)
            else {
                continue;
            };

            link.observe(route.connection, batch, slice.frames.clone());

            let mut frames = FrameReader::new(batch, slice.frames);
            let _ = transport.write(
                route,
                link.bundle.get(),
                BinaryWriter::NEWEST,
                usize::MAX,
                |encoder| {
                    while let Some(frame) = frames.raw()? {
                        encoder.raw(frame.kind, &batch[frame.payload])?;
                    }

                    Ok(())
                },
            );
        }
    }
}

impl<D: Driver> Future for Relay<D> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let relay = self.get_mut();

        loop {
            match Pin::new(&mut relay.receiver).poll_next(context) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(batch)) => {
                    let Some(transport) = relay.link.transport.upgrade() else {
                        return Poll::Ready(());
                    };

                    relay.forward(&transport, &batch);
                }
                Poll::Ready(None) => {
                    if let Some(transport) = relay.link.transport.upgrade() {
                        if let Some(connection) = relay.link.connection.take() {
                            relay.link.close_remote(
                                &transport,
                                connection,
                                ErrorCode::ConnectionLoss,
                            );
                        }

                        transport.detach(relay.handle);
                    }

                    return Poll::Ready(());
                }
            }
        }
    }
}

impl<D: Driver> RelayLink<D> {
    /// Updates current connection state from `route` and slice `frames` in `batch`.
    fn observe(&self, route: Option<ConnectionId>, batch: &[u8], frames: Range<usize>) {
        if route.is_some() {
            self.connection.set(route);
        }

        let mut reader = FrameReader::new(batch, frames);

        while let Ok(Some(frame)) = reader.raw() {
            match FrameKind::try_from(u32::from(frame.kind)) {
                Ok(FrameKind::Ack) => {
                    let payload = &mut BinaryReader::new(&batch[frame.payload]);

                    if let Ok(Control::Ack { connection, .. }) =
                        Control::read(FrameKind::Ack, payload)
                    {
                        self.connection.set(Some(connection));
                    }
                }
                Ok(FrameKind::Close) => self.connection.set(None),
                _ => {}
            }
        }
    }

    /// Sends a `Close` frame for `connection` to the transport peer for the spawned channel.
    fn close_remote(
        &self,
        transport: &TransportState<D>,
        connection: ConnectionId,
        reason: ErrorCode,
    ) {
        let route = Route {
            from: self.role,
            channel: self.channel,
            connection: Some(connection),
        };
        let _ = transport.write(
            route,
            self.bundle.get(),
            BinaryWriter::NEWEST,
            usize::MAX,
            |encoder| encoder.control(&Control::Close { reason }),
        );
    }
}

impl<D: Driver> Forward<D> {
    /// Closes the spawned channel port. If `connection` is `Some`, first sends a transport-peer `Close` frame.
    fn close_port(&self, connection: Option<ConnectionId>, reason: ErrorCode) {
        let Some(to) = self.to.borrow_mut().take() else {
            return;
        };
        let Some(connection) = connection else {
            return;
        };
        let mut frames = BinaryWriter::default();
        let mut encoder = FrameEncoder::new(&mut frames, BinaryWriter::NEWEST);
        let Some(last) = encoder
            .control(&Control::Close { reason })
            .ok()
            .and(encoder.last())
        else {
            return;
        };
        let key = SliceKey::NoAlias(Route {
            from: self.link.remote,
            channel: self.link.channel,
            connection: Some(connection),
        });
        let mut batch = self.batch.borrow_mut();
        let outcome = BatchWriter::new(&mut batch, CORE_MAJOR, ONE_SLICE)
            .and_then(|mut writer| writer.slice(key, frames.as_slice(), last));

        if outcome.is_ok() {
            let _ = to.unbounded_send(batch.as_slice().to_vec());
        }
    }
}

impl<D: Driver> Endpoint for Forward<D> {
    fn receive(self: Rc<Self>, inbound: Inbound<'_>) {
        let Some(to) = &*self.to.borrow() else {
            return;
        };

        self.link
            .observe(inbound.connection, inbound.batch, inbound.frames.clone());

        let mut batch = self.batch.borrow_mut();
        let route = Route {
            from: self.link.remote,
            channel: self.link.channel,
            connection: inbound.connection,
        };
        let outcome =
            BatchWriter::new(&mut batch, inbound.core, ONE_SLICE).and_then(|mut writer| {
                writer.last_slice(
                    SliceKey::NoAlias(route),
                    &inbound.batch[inbound.frames.clone()],
                )
            });

        if outcome.is_ok() {
            let _ = to.unbounded_send(batch.as_slice().to_vec());
        }
    }

    fn on_disconnect(&self) {
        self.close_port(self.link.connection.take(), ErrorCode::ConnectionLoss);
    }

    fn close(&self, reason: ErrorCode) {
        let connection = self.link.connection.take();

        if let (Some(connection), Some(transport)) = (connection, self.link.transport.upgrade()) {
            self.link.close_remote(&transport, connection, reason);
        }

        self.close_port(connection, reason);
    }
}
