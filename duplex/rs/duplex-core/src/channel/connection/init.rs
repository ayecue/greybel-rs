use super::pending_connection::OpenOutcome;
use super::{
    ConnectionActive, ConnectionContext, ConnectionFailure, ConnectionHandler, ConnectionState,
    Effect, NegotiatedOpen, Side,
};
use crate::binary::{BinaryDecode, BinaryEncode};
use crate::connection_id::ConnectionId;
use crate::definition::{CORE, ErrorCode};
use crate::error::Error;
use crate::frame::Control;
use crate::migration::Session;
use crate::protocol::Protocol;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Initial state while a client opens a connection or a provider listens.
pub struct ConnectionInit {
    pub(crate) id: u32,
}

impl ConnectionInit {
    #[must_use]
    /// Returns the client open-request ID, or 0 on a provider.
    pub fn id(&self) -> u32 {
        self.id
    }

    pub(crate) fn start<Pr: Protocol>(self, context: &mut ConnectionContext<Pr>) {
        context.reset_checks();

        match context.side {
            Side::Client { .. } => self.open(context),
            Side::Provider { .. } => context.post(Control::Listen, None),
        }
    }

    fn open<Pr: Protocol>(self, context: &mut ConnectionContext<Pr>) {
        let Side::Client { open_payload } = &context.side else {
            return;
        };
        let version = context.versions.version;
        let open = Control::Open {
            id: self.id,
            core: CORE,
            protocol: version,
            fingerprint: context
                .versions
                .fingerprint(version.minor)
                .unwrap_or_default(),
            payload: open_payload.clone(),
        };

        context.post(open, None);
    }

    fn on_ack<Pr: Protocol>(
        self,
        connection: ConnectionId,
        session: Session,
        fingerprint: u32,
        deprecation: Option<String>,
        payload: &[u8],
        context: &mut ConnectionContext<Pr>,
    ) -> ConnectionState<Pr> {
        let versions = context.versions;
        let agrees = session.core.is_within(CORE)
            && session.protocol.is_within(versions.version)
            && versions.fingerprint(session.protocol.minor) == Some(fingerprint);
        let ack = if agrees {
            Pr::Ack::from_bytes(payload).map_err(|_| ErrorCode::MalformedMessage)
        } else {
            Err(ErrorCode::VersionMismatch)
        };

        match ack {
            Ok(ack) => {
                let deprecation = deprecation.map(|note| (versions.version, note));

                ConnectionActive::connect(
                    connection,
                    self.id,
                    session,
                    Some(ack),
                    deprecation,
                    context,
                )
            }
            Err(reason) => {
                context.post(Control::Close { reason }, Some(connection));

                Self::fail(reason, None, context)
            }
        }
    }

    fn fail<Pr: Protocol>(
        reason: ErrorCode,
        refusal: Option<Pr::Refusal>,
        context: &mut ConnectionContext<Pr>,
    ) -> ConnectionState<Pr> {
        context.emit(Effect::End {
            connection: None,
            reason,
        });
        context.emit(Effect::Close(reason));

        context.enter(ConnectionFailure { reason, refusal }.into())
    }
}

impl<Pr: Protocol> From<ConnectionInit> for ConnectionState<Pr> {
    fn from(init: ConnectionInit) -> Self {
        Self::Init(init)
    }
}

impl<Pr: Protocol> ConnectionHandler<Pr> for ConnectionInit {
    fn current_session(&self) -> Result<Option<(ConnectionId, &Session)>, Error> {
        Ok(None)
    }

    fn on_control(
        self,
        control: Control,
        _: Option<ConnectionId>,
        context: &mut ConnectionContext<Pr>,
    ) -> ConnectionState<Pr> {
        if !context.is_client() {
            if let Control::Open {
                id,
                core,
                protocol,
                fingerprint,
                payload,
            } = control
            {
                context.negotiate(id, core, protocol, fingerprint, &payload, None);
            }

            return self.into();
        }

        match control {
            Control::Listen => {
                self.open(context);

                self.into()
            }
            Control::Ack {
                id,
                connection,
                core,
                protocol,
                fingerprint,
                deprecation,
                payload,
            } if id == self.id => {
                let session = Session {
                    core,
                    protocol,
                    migration_path: None,
                };

                self.on_ack(
                    connection,
                    session,
                    fingerprint,
                    deprecation,
                    &payload,
                    context,
                )
            }
            Control::Refuse {
                id,
                reason,
                payload,
                ..
            } if id == self.id => {
                let refusal = if reason == ErrorCode::Denial {
                    Pr::Refusal::from_bytes(&payload).ok()
                } else {
                    None
                };

                Self::fail(reason, refusal, context)
            }
            _ => self.into(),
        }
    }

    fn on_check(self, context: &mut ConnectionContext<Pr>) -> ConnectionState<Pr> {
        let Some(health_check) = context.health_check.filter(|_| context.is_client()) else {
            return self.into();
        };

        context.missed_checks += 1;
        if context.missed_checks > health_check.close_threshold {
            return Self::fail(ErrorCode::Timeout, None, context);
        }

        self.open(context);

        self.into()
    }

    fn on_open_result(
        self,
        id: u32,
        outcome: OpenOutcome<Pr>,
        random: [u8; 16],
        context: &mut ConnectionContext<Pr>,
    ) -> ConnectionState<Pr> {
        let Side::Provider {
            negotiated_open, ..
        } = &mut context.side
        else {
            return self.into();
        };
        let Some(NegotiatedOpen {
            session,
            fingerprint,
            deprecation,
            ..
        }) = negotiated_open.take_if(|negotiated_open| negotiated_open.id == id)
        else {
            return self.into();
        };
        let payload = match outcome {
            Ok(ack) => ack
                .to_bytes()
                .map_err(|_| (ErrorCode::MalformedMessage, Vec::new())),
            Err(Ok(refusal)) => match refusal.to_bytes() {
                Ok(payload) => Err((ErrorCode::Denial, payload)),
                Err(_) => Err((ErrorCode::MalformedMessage, Vec::new())),
            },
            Err(Err(reason)) => Err((reason, Vec::new())),
        };
        let payload = match payload {
            Ok(payload) => payload,
            Err((reason, payload)) => {
                context.refuse(id, reason, payload);

                return self.into();
            }
        };
        let connection = ConnectionContext::<Pr>::connection_id(random);
        let ack = Control::Ack {
            id,
            connection,
            core: session.core,
            protocol: session.protocol,
            fingerprint,
            deprecation: deprecation.as_ref().map(|(_, note)| note.clone()),
            payload,
        };

        context.post(ack.clone(), None);
        if let Side::Provider { ack: on_ack, .. } = &mut context.side {
            *on_ack = Some(ack);
        }

        ConnectionActive::connect(connection, id, session, None, deprecation, context)
    }
}
