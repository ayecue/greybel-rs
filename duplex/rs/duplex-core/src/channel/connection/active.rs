use super::{
    ConnectionContext, ConnectionEnd, ConnectionHandler, ConnectionInit, ConnectionState, Effect,
    Side,
};
use crate::connection_id::ConnectionId;
use crate::definition::{ErrorCode, Version};
use crate::error::Error;
use crate::frame::Control;
use crate::migration::Session;
use crate::protocol::Protocol;

/// State for an established connection.
pub struct ConnectionActive<Pr: Protocol> {
    id: ConnectionId,
    owner: u32,
    session: Session,
    ack: Option<Pr::Ack>,
    deprecation: Option<(Version, String)>,
}

impl<Pr: Protocol> Clone for ConnectionActive<Pr> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            owner: self.owner,
            session: self.session.clone(),
            ack: self.ack.clone(),
            deprecation: self.deprecation.clone(),
        }
    }
}

impl<Pr: Protocol> ConnectionActive<Pr> {
    #[must_use]
    /// Returns the provider-assigned connection ID.
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    #[must_use]
    /// Returns the protocol version selected during negotiation.
    pub fn version(&self) -> Version {
        self.session.protocol
    }

    #[must_use]
    /// Returns the provider acknowledgement payload on the client.
    pub fn ack(&self) -> Option<&Pr::Ack> {
        self.ack.as_ref()
    }

    #[must_use]
    /// Returns the deprecated protocol version and note, if available.
    pub fn deprecation(&self) -> Option<(Version, &str)> {
        self.deprecation
            .as_ref()
            .map(|(version, note)| (*version, note.as_str()))
    }

    pub(crate) fn session(&self) -> &Session {
        &self.session
    }

    pub(crate) fn connect(
        id: ConnectionId,
        owner: u32,
        session: Session,
        ack: Option<Pr::Ack>,
        deprecation: Option<(Version, String)>,
        context: &mut ConnectionContext<Pr>,
    ) -> ConnectionState<Pr> {
        context.reset_checks();
        context.emit(Effect::Connection {
            connection: id,
            session: session.clone(),
        });

        context.enter(
            Self {
                id,
                owner,
                session,
                ack,
                deprecation,
            }
            .into(),
        )
    }

    fn is_current(&self, connection: Option<ConnectionId>) -> bool {
        connection == Some(self.id)
    }

    fn end(
        self,
        reason: ErrorCode,
        notify_remote: bool,
        context: &mut ConnectionContext<Pr>,
    ) -> ConnectionState<Pr> {
        if notify_remote {
            context.post(Control::Close { reason }, Some(self.id));
        }

        context.emit(Effect::End {
            connection: Some(self.id),
            reason,
        });
        context.enter(ConnectionEnd { reason }.into());
        if let Side::Provider {
            ack: pending_ack, ..
        } = &mut context.side
        {
            *pending_ack = None;
        }

        let init = ConnectionInit { id: 0 };

        init.start(context);

        context.enter(init.into())
    }
}

impl<Pr: Protocol> From<ConnectionActive<Pr>> for ConnectionState<Pr> {
    fn from(active: ConnectionActive<Pr>) -> Self {
        Self::Active(active)
    }
}

impl<Pr: Protocol> ConnectionHandler<Pr> for ConnectionActive<Pr> {
    fn current_session(&self) -> Result<Option<(ConnectionId, &Session)>, Error> {
        Ok(Some((self.id, &self.session)))
    }

    fn on_control(
        self,
        control: Control,
        connection: Option<ConnectionId>,
        context: &mut ConnectionContext<Pr>,
    ) -> ConnectionState<Pr> {
        let current = self.is_current(connection);

        match (control, context.is_client()) {
            (Control::Pong, _) if current => {
                context.record_activity();

                self.into()
            }
            (Control::Ping, _) if current => {
                context.record_activity();
                context.post(Control::Pong, connection);

                self.into()
            }
            (Control::Close { reason }, true) if current => context.close(Some(self.id), reason),
            (Control::Close { reason }, false) if current => self.end(reason, false, context),
            (
                Control::Open {
                    id,
                    core,
                    protocol,
                    fingerprint,
                    payload,
                },
                false,
            ) => {
                context.negotiate(id, core, protocol, fingerprint, &payload, Some(self.owner));

                self.into()
            }
            _ => self.into(),
        }
    }

    fn on_check(self, context: &mut ConnectionContext<Pr>) -> ConnectionState<Pr> {
        let Some(health_check) = context.health_check else {
            return self.into();
        };

        if std::mem::take(&mut context.activity_since_check) {
            return self.into();
        }

        context.missed_checks += 1;
        if context.missed_checks <= health_check.close_threshold {
            context.post(Control::Ping, Some(self.id));

            return self.into();
        }

        if context.is_client() {
            return self.close(ErrorCode::Timeout, true, context);
        }

        self.end(ErrorCode::Timeout, true, context)
    }

    fn close(
        self,
        reason: ErrorCode,
        notify_remote: bool,
        context: &mut ConnectionContext<Pr>,
    ) -> ConnectionState<Pr> {
        if notify_remote {
            context.post(Control::Close { reason }, Some(self.id));
        }

        context.close(Some(self.id), reason)
    }
}
