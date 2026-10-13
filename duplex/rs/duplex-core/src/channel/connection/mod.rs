mod active;
mod context;
mod end;
mod health;
mod init;
mod negotiation;
mod pending_connection;
mod state;
mod versions;

use std::rc::Rc;

pub use active::ConnectionActive;
pub(crate) use context::{ConnectionContext, NegotiatedOpen, Side};
pub use end::{ConnectionEnd, ConnectionFailure};
pub use health::Health;
pub use init::ConnectionInit;
pub(crate) use pending_connection::OpenOutcome;
pub use pending_connection::PendingConnection;
pub(crate) use state::ConnectionHandler;
pub use state::ConnectionState;
pub(crate) use versions::VersionTable;

use crate::channel::HealthCheck;
use crate::connection_id::ConnectionId;
use crate::definition::ErrorCode;
use crate::error::Error;
use crate::frame::Control;
use crate::migration::{MigrationStep, MigrationTable, Session};
use crate::protocol::Protocol;

pub(crate) enum Effect<Pr: Protocol> {
    Post {
        control: Control,
        connection: Option<ConnectionId>,
    },
    Connection {
        connection: ConnectionId,
        session: Session,
    },
    Open {
        id: u32,
        open: Pr::Open,
    },
    End {
        connection: Option<ConnectionId>,
        reason: ErrorCode,
    },
    Close(ErrorCode),
    Health(Health),
    Enter(ConnectionState<Pr>),
}

pub(crate) struct Connection<Pr: Protocol> {
    state: ConnectionState<Pr>,
    context: ConnectionContext<Pr>,
}

pub(crate) type EffectContainer<Pr> = Vec<Effect<Pr>>;

impl<Pr: Protocol> Connection<Pr> {
    pub(crate) fn client(
        id: u32,
        versions: VersionTable,
        open_payload: Vec<u8>,
        health_check: Option<HealthCheck>,
    ) -> Self {
        Self::new(id, versions, health_check, Side::Client { open_payload })
    }

    pub(crate) fn provider(versions: VersionTable, health_check: Option<HealthCheck>) -> Self {
        let side = Side::Provider {
            ack: None,
            negotiated_open: None,
            migrations: MigrationTable::default(),
        };

        Self::new(0, versions, health_check, side)
    }

    fn new(id: u32, versions: VersionTable, health_check: Option<HealthCheck>, side: Side) -> Self {
        Self {
            state: ConnectionInit { id }.into(),
            context: ConnectionContext::new(side, versions, health_check),
        }
    }

    pub(crate) fn state(&self) -> &ConnectionState<Pr> {
        &self.state
    }

    pub(crate) fn start(&mut self) -> EffectContainer<Pr> {
        let ConnectionState::Init(init) = &self.state else {
            return Vec::new();
        };
        let init = *init;

        init.start(&mut self.context);
        self.context.enter(init.into());

        self.drain()
    }

    pub(crate) fn on_control(
        &mut self,
        control: Control,
        connection: Option<ConnectionId>,
    ) -> EffectContainer<Pr> {
        self.transition(|state, context| state.on_control(control, connection, context))
    }

    pub(crate) fn check(&mut self) -> EffectContainer<Pr> {
        self.transition(ConnectionState::on_check)
    }

    pub(crate) fn on_open_result(
        &mut self,
        id: u32,
        outcome: OpenOutcome<Pr>,
        random: [u8; 16],
    ) -> EffectContainer<Pr> {
        self.transition(|state, context| state.on_open_result(id, outcome, random, context))
    }

    pub(crate) fn close(&mut self, reason: ErrorCode, notify_remote: bool) -> EffectContainer<Pr> {
        self.transition(|state, context| state.close(reason, notify_remote, context))
    }

    pub(crate) fn on_activity(&mut self) -> EffectContainer<Pr> {
        if self.context.missed_checks == 0 {
            self.context.activity_since_check = true;

            return Vec::new();
        }

        let before = self.context.health(&self.state);

        self.context.record_activity();
        self.emit_health_change(before);

        self.drain()
    }

    pub(crate) fn migrate(&mut self, migration: Rc<dyn MigrationStep>) {
        if let Side::Provider { migrations, .. } = &mut self.context.side {
            migrations.add(migration);
        }
    }

    pub(crate) fn current_session(&self) -> Result<Option<(ConnectionId, Session)>, Error> {
        self.state
            .current_session()
            .map(|route| route.map(|(connection, session)| (connection, session.clone())))
    }

    pub(crate) fn current(&self) -> Option<ConnectionId> {
        match &self.state {
            ConnectionState::Active(active) => Some(active.id()),
            _ => None,
        }
    }

    pub(crate) fn session(&self) -> Option<&Session> {
        match &self.state {
            ConnectionState::Active(active) => Some(active.session()),
            _ => None,
        }
    }

    pub(crate) fn reason(&self) -> Option<ErrorCode> {
        self.state.reason()
    }

    pub(crate) fn health(&self) -> Health {
        self.context.health(&self.state)
    }

    fn transition(
        &mut self,
        transition: impl FnOnce(ConnectionState<Pr>, &mut ConnectionContext<Pr>) -> ConnectionState<Pr>,
    ) -> EffectContainer<Pr> {
        let before = self.health();
        let state = std::mem::replace(
            &mut self.state,
            ConnectionEnd {
                reason: ErrorCode::ClosedChannel,
            }
            .into(),
        );

        self.state = transition(state, &mut self.context);
        self.emit_health_change(before);

        self.drain()
    }

    fn emit_health_change(&mut self, before: Health) {
        let health = self.health();

        if health != before && self.current().is_some() {
            self.context.emit(Effect::Health(health));
        }
    }

    fn drain(&mut self) -> EffectContainer<Pr> {
        self.context.drain()
    }
}
