use super::{ConnectionContext, ConnectionHandler, ConnectionState};
use crate::connection_id::ConnectionId;
use crate::definition::ErrorCode;
use crate::error::Error;
use crate::migration::Session;
use crate::protocol::Protocol;

/// Failure state for a connection that did not become active.
pub struct ConnectionFailure<Pr: Protocol> {
    pub(crate) reason: ErrorCode,
    pub(crate) refusal: Option<Pr::Refusal>,
}

impl<Pr: Protocol> Clone for ConnectionFailure<Pr> {
    fn clone(&self) -> Self {
        Self {
            reason: self.reason,
            refusal: self.refusal.clone(),
        }
    }
}

impl<Pr: Protocol> ConnectionFailure<Pr> {
    #[must_use]
    /// Returns why the connection failed.
    pub fn reason(&self) -> ErrorCode {
        self.reason
    }

    #[must_use]
    /// Returns the provider refusal payload, if supplied.
    pub fn refusal(&self) -> Option<&Pr::Refusal> {
        self.refusal.as_ref()
    }
}

impl<Pr: Protocol> From<ConnectionFailure<Pr>> for ConnectionState<Pr> {
    fn from(failure: ConnectionFailure<Pr>) -> Self {
        Self::Failure(failure)
    }
}

impl<Pr: Protocol> ConnectionHandler<Pr> for ConnectionFailure<Pr> {
    fn current_session(&self) -> Result<Option<(ConnectionId, &Session)>, Error> {
        Err(Error::ChannelEnd(self.reason))
    }

    fn close(self, _: ErrorCode, _: bool, _: &mut ConnectionContext<Pr>) -> ConnectionState<Pr> {
        self.into()
    }

    fn reason(&self) -> Option<ErrorCode> {
        Some(self.reason)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// State for a closed connection.
pub struct ConnectionEnd {
    pub(crate) reason: ErrorCode,
}

impl ConnectionEnd {
    #[must_use]
    /// Returns why the connection closed.
    pub fn reason(&self) -> ErrorCode {
        self.reason
    }
}

impl<Pr: Protocol> From<ConnectionEnd> for ConnectionState<Pr> {
    fn from(final_state: ConnectionEnd) -> Self {
        Self::End(final_state)
    }
}

impl<Pr: Protocol> ConnectionHandler<Pr> for ConnectionEnd {
    fn current_session(&self) -> Result<Option<(ConnectionId, &Session)>, Error> {
        Err(Error::ChannelEnd(self.reason))
    }

    fn close(self, _: ErrorCode, _: bool, _: &mut ConnectionContext<Pr>) -> ConnectionState<Pr> {
        self.into()
    }

    fn reason(&self) -> Option<ErrorCode> {
        Some(self.reason)
    }
}
