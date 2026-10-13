use super::pending_connection::OpenOutcome;
use super::{
    ConnectionActive, ConnectionContext, ConnectionEnd, ConnectionFailure, ConnectionInit,
};
use crate::connection_id::ConnectionId;
use crate::definition::ErrorCode;
use crate::error::Error;
use crate::frame::Control;
use crate::migration::Session;
use crate::protocol::Protocol;

/// Channel connection state.
pub enum ConnectionState<Pr: Protocol> {
    /// A client is opening or a provider is listening.
    Init(ConnectionInit),
    /// The connection is active.
    Active(ConnectionActive<Pr>),
    /// The connection failed or was refused before activation.
    Failure(ConnectionFailure<Pr>),
    /// The connection has closed.
    End(ConnectionEnd),
}

impl<Pr: Protocol> Clone for ConnectionState<Pr> {
    fn clone(&self) -> Self {
        match self {
            Self::Init(init) => Self::Init(*init),
            Self::Active(active) => Self::Active(active.clone()),
            Self::Failure(failure) => Self::Failure(failure.clone()),
            Self::End(final_state) => Self::End(*final_state),
        }
    }
}

pub(crate) trait ConnectionHandler<Pr: Protocol>: Sized + Into<ConnectionState<Pr>> {
    fn current_session(&self) -> Result<Option<(ConnectionId, &Session)>, Error>;

    fn on_control(
        self,
        _control: Control,
        _connection: Option<ConnectionId>,
        _context: &mut ConnectionContext<Pr>,
    ) -> ConnectionState<Pr> {
        self.into()
    }

    fn on_check(self, _context: &mut ConnectionContext<Pr>) -> ConnectionState<Pr> {
        self.into()
    }

    fn on_open_result(
        self,
        _id: u32,
        _outcome: OpenOutcome<Pr>,
        _random: [u8; 16],
        _context: &mut ConnectionContext<Pr>,
    ) -> ConnectionState<Pr> {
        self.into()
    }

    fn close(
        self,
        reason: ErrorCode,
        _notify_remote: bool,
        context: &mut ConnectionContext<Pr>,
    ) -> ConnectionState<Pr> {
        context.close(None, reason)
    }

    fn reason(&self) -> Option<ErrorCode> {
        None
    }
}

macro_rules! each_state {
    ($state:expr, |$handler:ident| $call:expr) => {
        match $state {
            ConnectionState::Init($handler) => $call,
            ConnectionState::Active($handler) => $call,
            ConnectionState::Failure($handler) => $call,
            ConnectionState::End($handler) => $call,
        }
    };
}

impl<Pr: Protocol> ConnectionState<Pr> {
    #[must_use]
    /// Returns the failure or close reason, or `None` while the connection has not ended.
    pub fn reason(&self) -> Option<ErrorCode> {
        each_state!(self, |state| ConnectionHandler::<Pr>::reason(state))
    }

    pub(crate) fn current_session(&self) -> Result<Option<(ConnectionId, &Session)>, Error> {
        each_state!(self, |state| ConnectionHandler::<Pr>::current_session(
            state
        ))
    }

    pub(crate) fn on_control(
        self,
        control: Control,
        connection: Option<ConnectionId>,
        context: &mut ConnectionContext<Pr>,
    ) -> Self {
        each_state!(self, |state| state.on_control(control, connection, context))
    }

    pub(crate) fn on_check(self, context: &mut ConnectionContext<Pr>) -> Self {
        each_state!(self, |state| state.on_check(context))
    }

    pub(crate) fn on_open_result(
        self,
        id: u32,
        outcome: OpenOutcome<Pr>,
        random: [u8; 16],
        context: &mut ConnectionContext<Pr>,
    ) -> Self {
        each_state!(self, |state| state
            .on_open_result(id, outcome, random, context))
    }

    pub(crate) fn close(
        self,
        reason: ErrorCode,
        notify_remote: bool,
        context: &mut ConnectionContext<Pr>,
    ) -> Self {
        each_state!(self, |state| state.close(reason, notify_remote, context))
    }
}
