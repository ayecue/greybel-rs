mod close;
mod connection;
mod effects;
mod exchange;
mod fragments;
mod hooks;
mod inbound;
mod io;
pub(crate) mod message;
mod options;
mod response;
mod router;
mod sender;
mod state;
mod timeouts;

use std::rc::Rc;

pub use close::CloseFuture;
pub(crate) use connection::VersionTable;
pub use connection::{
    ConnectionActive, ConnectionEnd, ConnectionFailure, ConnectionInit, ConnectionState, Health,
    PendingConnection,
};
pub use exchange::{
    IncomingExchange, IncomingStream, ItemWriter, OutgoingExchange, PendingResponse, Responder,
    ResponseStream, StreamResponder, StreamWriter,
};
pub use hooks::HookId;
pub(crate) use options::ChannelSettings;
pub use options::{
    Bundle, ChannelOptions, DEFAULT_MAX_MESSAGE, DEFAULT_STREAM_WINDOW, HealthCheck, HealthChecks,
    Limits, SendOptions, TransportFlavour,
};
pub use router::{NextMessage, NextNotification};
use state::ChannelState;

use crate::binary::{BinaryEncode, ValidatedDecode};
use crate::connection_id::ConnectionId;
use crate::definition::{ErrorCode, Version};
use crate::driver::Driver;
use crate::error::Error;
use crate::migration::Migration;
use crate::protocol::{Exchange, Message, NotificationMessage, Peer, Protocol};
use crate::transport::Transport;

type ResponderOf<M> = <<M as Message>::Exchange as Exchange>::Responder;
type HandleOf<M> = <<M as Message>::Exchange as Exchange>::Handle;

/// Protocol `P` channel for one peer using driver `D` transport.
///
/// Clones share one channel. Dropping the last clone closes it.
pub struct Channel<P: Peer, D: Driver>(Rc<Attachment<P, D>>);

struct Attachment<P: Peer, D: Driver> {
    transport: Transport<D>,
    channel: Rc<ChannelState<P, D>>,
}

impl<P: Peer, D: Driver> Channel<P, D> {
    /// Attaches a `P` channel to `transport` and schedules client connection opening.
    ///
    /// # Panics
    ///
    /// Panics if the transport already has a `P` protocol channel.
    #[must_use]
    pub fn new(transport: &Transport<D>, options: ChannelOptions) -> Self
    where
        P: Peer<OpenPayload = ()>,
    {
        Self::init(transport, options, Vec::new())
    }

    /// Calls [`Channel::new`] with `open_value` as the protocol `Open` payload.
    ///
    /// # Panics
    ///
    /// Panics if the transport already has a `P` protocol channel.
    ///
    /// # Errors
    ///
    /// Returns `Encode` if `open_value` cannot be encoded.
    pub fn open(
        transport: &Transport<D>,
        options: ChannelOptions,
        open_value: &P::OpenPayload,
    ) -> Result<Self, Error> {
        let open_payload = open_value.to_bytes()?;

        Ok(Self::init(transport, options, open_payload))
    }

    fn init(transport: &Transport<D>, options: ChannelOptions, open_payload: Vec<u8>) -> Self {
        const {
            assert!(
                D::CAPABILITIES.supports(P::ROLE, <P::Protocol as Protocol>::REQUIRES),
                "the driver's transports can't carry the protocol's channels"
            );
        }

        let channel = ChannelState::attach(transport.state(), options, open_payload);

        Self(Rc::new(Attachment {
            transport: transport.clone(),
            channel,
        }))
    }

    /// Returns the attached transport.
    pub fn transport(&self) -> &Transport<D> {
        &self.0.transport
    }

    /// Returns the transport driver.
    pub fn driver(&self) -> &D {
        self.0.transport.driver()
    }

    #[must_use]
    /// Returns current connection state.
    pub fn state(&self) -> ConnectionState<P::Protocol> {
        self.0.channel.state()
    }

    #[must_use]
    /// Returns the active connection ID, or `None` if no connection is active.
    pub fn connection(&self) -> Option<ConnectionId> {
        self.0.channel.current_connection()
    }

    #[must_use]
    /// Returns the protocol version selected for the active connection, or `None` if inactive.
    pub fn version(&self) -> Option<Version> {
        self.0.channel.version()
    }

    /// Registers an `include_migration!` migration for providers serving older major versions.
    pub fn migrate(&self, migration: impl Migration) {
        self.0.channel.migrate(Rc::new(migration));
    }

    /// Sets the provider handler for accepting or denying client open requests.
    pub fn on_connect(
        &self,
        handler: impl Fn(<P::Protocol as Protocol>::Open, PendingConnection<P::Protocol>) + 'static,
    ) {
        self.0.channel.on_connect(Rc::new(handler));
    }

    /// Runs `hook` when the connection enters `ConnectionInit`.
    /// Pass the returned ID to [`Channel::remove_hook`] to unregister it.
    pub fn on_open(&self, hook: impl Fn(&ConnectionInit) + 'static) -> HookId {
        self.0.channel.hooks.on_open(Rc::new(hook))
    }

    /// Runs `hook` when the connection becomes active.
    pub fn on_ack(&self, hook: impl Fn(&ConnectionActive<P::Protocol>) + 'static) -> HookId {
        self.0.channel.hooks.on_ack(Rc::new(hook))
    }

    /// Runs `hook` when a client connection fails before activation.
    pub fn on_failure(&self, hook: impl Fn(&ConnectionFailure<P::Protocol>) + 'static) -> HookId {
        self.0.channel.hooks.on_failure(Rc::new(hook))
    }

    /// Runs `hook` when the channel closes.
    pub fn on_close(&self, hook: impl Fn(&ConnectionEnd) + 'static) -> HookId {
        self.0.channel.hooks.on_close(Rc::new(hook))
    }

    /// Runs `hook` when connection health changes.
    pub fn on_health(&self, hook: impl Fn(Health) + 'static) -> HookId {
        self.0.channel.hooks.on_health(Rc::new(hook))
    }

    /// Unregisters hook `id`.
    pub fn remove_hook(&self, id: HookId) {
        self.0.channel.hooks.remove(id);
    }

    #[must_use]
    /// Returns current connection health.
    pub fn health(&self) -> Health {
        self.0.channel.health()
    }

    #[must_use]
    /// Returns the close reason, or `None` if the channel remains open.
    pub fn close_reason(&self) -> Option<ErrorCode> {
        self.0.channel.close_reason()
    }

    /// Returns a future resolving to the channel close reason.
    pub fn close_event(&self) -> CloseFuture {
        self.0.channel.close_event()
    }

    /// Closes the channel using `reason` and notifies the remote peer.
    pub fn close(&self, reason: ErrorCode) {
        self.0.channel.close(reason);
    }

    /// Runs the health check used by the scheduled interval.
    pub fn check(&self) {
        self.0.channel.check();
    }

    /// Sends `message`, queuing it until a connection is available.
    /// Returns the handle for the message exchange.
    ///
    /// # Errors
    ///
    /// Returns `Invalid` if the message violates a declared requirement, `Encode` if encoding fails, `Unavailable` if
    /// the negotiated version lacks the message, `Rejection` with `Overload` if too many messages are pending, and
    /// `ChannelEnd` if the channel has closed.
    pub fn send<M: Message<Sender = P>>(&self, message: &M) -> Result<HandleOf<M>, Error> {
        self.0.channel.send(message, SendOptions::default())
    }

    /// Calls [`Channel::send`] with per-message `options`.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Channel::send`].
    pub fn send_with<M: Message<Sender = P>>(
        &self,
        message: &M,
        options: SendOptions,
    ) -> Result<HandleOf<M>, Error> {
        self.0.channel.send(message, options)
    }

    /// Appends `handler` to the notification `N` handler chain.
    /// Use `into_owned` to convert borrowed notification values to owned values.
    pub fn on<N>(&self, handler: impl for<'a> Fn(N::Ref<'a>, NextNotification<N>) + 'static)
    where
        N: NotificationMessage<Sender = P::Remote> + 'static,
    {
        self.0.channel.handlers.on(handler);
    }

    /// Registers `handler` for messages `M`; the handler uses the supplied responder. Notifications have no responder
    /// and must use [`Channel::on`]. Registering a notification here causes a compile-time error.
    pub fn handle<M>(&self, handler: impl Fn(M, ResponderOf<M>, NextMessage<M>) + 'static)
    where
        M: Message<Sender = P::Remote> + ValidatedDecode + 'static,
    {
        const {
            assert!(
                <M::Exchange as Exchange>::RESPONDS,
                "a notification's handlers are registered with `Channel::on`"
            );
        }

        self.0.channel.handlers.handle(handler);
    }
}

impl<P: Peer, D: Driver> Clone for Channel<P, D> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<P: Peer, D: Driver> Drop for Attachment<P, D> {
    fn drop(&mut self) {
        self.channel.close(ErrorCode::ClosedChannel);
        self.transport.state().detach(self.channel.endpoint());
    }
}
