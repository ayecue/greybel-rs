use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

use futures_channel::oneshot;
use scheduler::{Clock, Scheduler};

use super::close::CloseFuture;
use super::connection::{Connection, ConnectionState, Health, VersionTable};
use super::exchange::ChannelIo;
use super::fragments::Reassembly;
use super::hooks::HookManager;
use super::message::MessageTable;
use super::router::{ConnectDispatch, HandlerTable};
use super::{Bundle, ChannelOptions, ChannelSettings};
use crate::TimerOf;
use crate::binary::ValidatedEncode;
use crate::connection_id::ConnectionId;
use crate::definition::{ErrorCode, Role, Version};
use crate::driver::Driver;
use crate::error::Error;
use crate::frame::{Control, FrameEncoder, Route};
use crate::keys::{EndpointId, EntryId, Work};
use crate::migration::{MigrationStep, Session};
use crate::protocol::{MessageRequirementTable, MessageSegment, Peer, Protocol};
use crate::transport::{Endpoint, Inbound, TransportState};

pub(crate) struct ChannelState<P: Peer, D: Driver> {
    pub(super) this: Weak<Self>,
    pub(super) transport: Weak<TransportState<D>>,
    endpoint: EndpointId,
    pub(super) health_checks: RefCell<Option<TimerOf<D>>>,
    pub(super) settings: ChannelSettings,
    pub(super) connection: RefCell<Connection<P::Protocol>>,
    pub(super) messages: RefCell<MessageTable>,
    pub(super) handlers: HandlerTable<P, D>,
    pub(super) hooks: HookManager<P::Protocol>,
    pub(super) close_emitters: RefCell<Vec<oneshot::Sender<ErrorCode>>>,
    pub(super) fragments: RefCell<Option<Reassembly>>,
    pub(super) deadline: RefCell<Option<(Duration, TimerOf<D>)>>,
    active: Cell<Option<Active>>,
}

/// Routing cache for the current connection and negotiated session.
#[derive(Clone, Copy)]
struct Active {
    connection: ConnectionId,
    core: Version,
    protocol: Version,
    migrates: bool,
}

pub(crate) struct FrameTo {
    pub(crate) connection: ConnectionId,
    pub(crate) bundle: Bundle,
    pub(crate) minor: u16,
    pub(crate) starts: bool,
}

impl<P: Peer, D: Driver> ChannelState<P, D> {
    pub(crate) const NEWEST: u16 = <P::Protocol as Protocol>::VERSION.minor;
    pub(crate) const VALIDATES_INPUT: bool = matches!(P::ROLE, Role::Provider);
    const VERSIONS: VersionTable = VersionTable {
        version: <P::Protocol as Protocol>::VERSION,
        versions: <P::Protocol as Protocol>::VERSIONS,
    };

    pub(crate) fn attach(
        transport: &Rc<TransportState<D>>,
        options: ChannelOptions,
        open_payload: Vec<u8>,
    ) -> Rc<Self> {
        let settings = ChannelSettings::of(
            &options,
            <P::Protocol as Protocol>::FLAVOURS,
            transport.driver().transport_name(),
            D::STREAM_WINDOW,
        );
        let health_check = settings.health_check;
        let channel =
            transport.attach(<P::Protocol as Protocol>::CHANNEL_ID, P::ROLE, |endpoint| {
                let connection = match P::ROLE {
                    Role::Client => {
                        let [.., a, b, c, d] = transport.driver().random();
                        let id = u32::from_le_bytes([a, b, c, d]);

                        Connection::client(id, Self::VERSIONS, open_payload, health_check)
                    }
                    Role::Provider => Connection::provider(Self::VERSIONS, health_check),
                };

                Rc::new_cyclic(|this| Self {
                    this: Weak::clone(this),
                    transport: transport.weak(),
                    endpoint,
                    health_checks: RefCell::new(None),
                    settings,
                    connection: RefCell::new(connection),
                    messages: RefCell::default(),
                    handlers: HandlerTable::default(),
                    hooks: HookManager::default(),
                    close_emitters: RefCell::default(),
                    fragments: RefCell::default(),
                    deadline: RefCell::new(None),
                    active: Cell::new(None),
                })
            });
        let this = Rc::downgrade(&channel);

        transport.driver().scheduler().defer(Box::new(move || {
            if let Some(channel) = this.upgrade() {
                channel.start();
            }
        }));

        channel
    }

    fn start(&self) {
        let Some(transport) = self
            .transport
            .upgrade()
            .filter(|_| self.close_reason().is_none())
        else {
            return;
        };
        let secure = transport.matches_security(
            <P::Protocol as Protocol>::SECURITY,
            <P::Protocol as Protocol>::SIGNED,
        );
        let end_reason = if secure {
            transport.reason()
        } else {
            Some(ErrorCode::Insecure)
        };

        if let Some(reason) = end_reason {
            let effects = self.connection.borrow_mut().close(reason, false);

            self.apply(effects);

            return;
        }

        if let Some(health_check) = self.settings.health_check {
            let this = Weak::clone(&self.this);
            let timer = transport.driver().scheduler().interval(
                health_check.interval,
                Box::new(move || {
                    if let Some(channel) = this.upgrade() {
                        channel.check();
                    }
                }),
            );

            self.health_checks.replace(Some(timer));
        }

        let effects = self.connection.borrow_mut().start();

        self.apply(effects);
    }

    pub(crate) fn exchange_channel(&self) -> Weak<dyn ChannelIo> {
        Weak::clone(&self.this) as Weak<dyn ChannelIo>
    }

    pub(crate) fn endpoint(&self) -> EndpointId {
        self.endpoint
    }

    pub(crate) fn state(&self) -> ConnectionState<P::Protocol> {
        self.connection.borrow().state().clone()
    }

    pub(crate) fn current_connection(&self) -> Option<ConnectionId> {
        self.active.get().map(|active| active.connection)
    }

    /// Updates the routing cache after a connection state transition.
    pub(super) fn refresh_active(&self) {
        let state = self.connection.borrow();
        let active = state
            .current()
            .zip(state.session())
            .map(|(connection, session)| Active {
                connection,
                core: session.core,
                protocol: session.protocol,
                migrates: session.migration_path.is_some(),
            });

        self.active.set(active);
    }

    /// Returns current connection and session, or `Ok(None)` without an active session.
    pub(crate) fn active_target(&self) -> Result<Option<(ConnectionId, Session)>, Error> {
        match self.active.get() {
            Some(active) if !active.migrates => Ok(Some((active.connection, active.session()))),
            _ => self.connection.borrow().current_session(),
        }
    }

    pub(crate) fn version(&self) -> Option<Version> {
        self.connection
            .borrow()
            .session()
            .map(|session| session.protocol)
    }

    pub(crate) fn health(&self) -> Health {
        self.connection.borrow().health()
    }

    pub(crate) fn close_reason(&self) -> Option<ErrorCode> {
        self.connection.borrow().reason()
    }

    pub(crate) fn migrate(&self, migration: Rc<dyn MigrationStep>) {
        self.connection.borrow_mut().migrate(migration);
    }

    pub(crate) fn on_connect(&self, handler: ConnectDispatch<P::Protocol>) {
        self.handlers.on_connect(handler);
    }

    pub(crate) fn close_event(&self) -> CloseFuture {
        if let Some(reason) = self.close_reason() {
            return CloseFuture::ready(reason);
        }

        let (sender, receiver) = oneshot::channel();

        self.close_emitters.borrow_mut().push(sender);

        CloseFuture::from_receiver(receiver)
    }

    pub(crate) fn close(&self, reason: ErrorCode) {
        let effects = self.connection.borrow_mut().close(reason, true);

        self.apply(effects);
        if let Some(transport) = self.transport.upgrade() {
            transport.flush_all();
        }
    }

    pub(crate) fn check(&self) {
        let effects = self.connection.borrow_mut().check();

        self.apply(effects);
    }

    pub(crate) fn on_activity(&self) {
        let effects = self.connection.borrow_mut().on_activity();

        if !effects.is_empty() {
            self.apply(effects);
        }
    }

    pub(crate) fn on_control(&self, control: Control, connection: Option<ConnectionId>) {
        let effects = self.connection.borrow_mut().on_control(control, connection);

        self.apply(effects);
    }

    pub(crate) fn now(&self) -> Option<Duration> {
        Some(self.transport.upgrade()?.driver().scheduler().now())
    }

    /// Returns requirements for inbound message type ID `type_id`.
    pub(crate) fn inbound_requirements(type_id: u32) -> &'static MessageRequirementTable {
        <P::Remote as Peer>::SEND_REQUIREMENTS
            .get(type_id as usize)
            .copied()
            .unwrap_or(&MessageRequirementTable::NONE)
    }

    /// Returns requirements for outbound message type ID `type_id`.
    pub(crate) fn outbound_requirements(type_id: u32) -> &'static MessageRequirementTable {
        P::SEND_REQUIREMENTS
            .get(type_id as usize)
            .copied()
            .unwrap_or(&MessageRequirementTable::NONE)
    }

    pub(crate) fn limit_of(&self, requirements: &MessageRequirementTable) -> usize {
        if self.settings.validating && P::ROLE == Role::Client {
            requirements.limit as usize
        } else {
            usize::MAX
        }
    }

    pub(crate) fn validate_segment(
        &self,
        requirements: &MessageRequirementTable,
        segment: MessageSegment,
        value: &dyn ValidatedEncode,
        item_count: u32,
    ) -> Result<(), Error> {
        if !self.settings.validating {
            return Ok(());
        }

        value.validate(requirements.of(segment))?;

        match segment {
            MessageSegment::MessageItem | MessageSegment::ResponseItem => requirements
                .items_of(segment)
                .is_less_than(item_count as usize + 1),
            MessageSegment::MessageEnd => requirements
                .items_of(MessageSegment::MessageItem)
                .is_within(item_count as usize),
            MessageSegment::ResponseEnd => requirements
                .items_of(MessageSegment::ResponseItem)
                .is_within(item_count as usize),
            _ => Ok(()),
        }
    }

    /// Appends a frame to the transport outbox in write order.
    pub(crate) fn write_frame(
        &self,
        bundle: Bundle,
        connection: Option<ConnectionId>,
        minor: u16,
        write: impl FnOnce(&mut FrameEncoder<'_>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let transport = self
            .transport
            .upgrade()
            .ok_or(Error::ChannelEnd(ErrorCode::ConnectionLoss))?;

        transport.write(
            Self::route_of(connection),
            bundle,
            minor,
            self.settings.max_message,
            write,
        )
    }

    /// Writes the next frame for message `id` to the transport outbox.
    /// Writing the first frame moves the message to response-pending state.
    pub(crate) fn write_message_frame(
        &self,
        id: EntryId,
        to: &FrameTo,
        write: impl FnOnce(&mut FrameEncoder<'_>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.write_frame(to.bundle, Some(to.connection), to.minor, write)?;
        if to.starts {
            self.on_dispatch(id);
        }

        Ok(())
    }

    pub(super) fn has_queued_work(&self) -> bool {
        self.transport
            .upgrade()
            .is_some_and(|transport| transport.has_queued_work())
    }

    /// Returns whether `connection` is current and has an active session.
    pub(crate) fn is_current(&self, connection: ConnectionId) -> bool {
        self.active
            .get()
            .is_some_and(|active| active.connection == connection)
    }

    pub(crate) fn session_of(&self, connection: ConnectionId) -> Option<Session> {
        let active = self
            .active
            .get()
            .filter(|active| active.connection == connection)?;

        if !active.migrates {
            return Some(active.session());
        }

        self.connection.borrow().session().cloned()
    }

    pub(crate) fn route_of(connection: Option<ConnectionId>) -> Route {
        Route {
            from: P::ROLE,
            channel: <P::Protocol as Protocol>::CHANNEL_ID,
            connection,
        }
    }
}

impl Active {
    fn session(self) -> Session {
        Session {
            core: self.core,
            protocol: self.protocol,
            migration_path: None,
        }
    }
}

impl<P: Peer, D: Driver> Endpoint for ChannelState<P, D> {
    fn receive(self: Rc<Self>, inbound: Inbound<'_>) {
        self.read_slice(inbound);
    }

    fn process(&self, work: Work) {
        match work {
            Work::Message(id) => self.handle_message(id),
            Work::Notification(id) => self.handle_notification(id),
            Work::Settle(id) => self.handle_settle(id),
        }
    }

    fn on_disconnect(&self) {
        let effects = self
            .connection
            .borrow_mut()
            .close(ErrorCode::ConnectionLoss, false);

        self.apply(effects);
    }

    fn close(&self, reason: ErrorCode) {
        Self::close(self, reason);
    }
}
