use super::{ConnectionEnd, ConnectionState, Effect, Health, VersionTable};
use crate::channel::HealthCheck;
use crate::connection_id::ConnectionId;
use crate::definition::{ErrorCode, Version};
use crate::frame::Control;
use crate::migration::{MigrationTable, Session};
use crate::protocol::Protocol;

pub(crate) struct ConnectionContext<Pr: Protocol> {
    pub(crate) side: Side,
    pub(crate) versions: VersionTable,
    pub(crate) health_check: Option<HealthCheck>,
    pub(crate) missed_checks: usize,
    pub(crate) activity_since_check: bool,
    effects: Vec<Effect<Pr>>,
}

pub(crate) enum Side {
    Client {
        open_payload: Vec<u8>,
    },
    Provider {
        ack: Option<Control>,
        negotiated_open: Option<NegotiatedOpen>,
        migrations: MigrationTable,
    },
}

pub(crate) struct NegotiatedOpen {
    pub(crate) id: u32,
    pub(crate) session: Session,
    pub(crate) fingerprint: u32,
    pub(crate) deprecation: Option<(Version, String)>,
}

impl<Pr: Protocol> ConnectionContext<Pr> {
    pub(crate) fn new(
        side: Side,
        versions: VersionTable,
        health_check: Option<HealthCheck>,
    ) -> Self {
        Self {
            side,
            versions,
            health_check,
            missed_checks: 0,
            activity_since_check: false,
            effects: Vec::new(),
        }
    }

    pub(crate) fn record_activity(&mut self) {
        self.missed_checks = 0;
        self.activity_since_check = true;
    }

    pub(crate) fn reset_checks(&mut self) {
        self.missed_checks = 0;
        self.activity_since_check = false;
    }

    pub(crate) fn drain(&mut self) -> Vec<Effect<Pr>> {
        std::mem::take(&mut self.effects)
    }

    pub(crate) fn post(&mut self, control: Control, connection: Option<ConnectionId>) {
        self.emit(Effect::Post {
            control,
            connection,
        });
    }

    pub(crate) fn emit(&mut self, effect: Effect<Pr>) {
        self.effects.push(effect);
    }

    pub(crate) fn is_client(&self) -> bool {
        matches!(self.side, Side::Client { .. })
    }

    pub(crate) fn close(
        &mut self,
        connection: Option<ConnectionId>,
        reason: ErrorCode,
    ) -> ConnectionState<Pr> {
        self.emit(Effect::End { connection, reason });
        self.emit(Effect::Close(reason));

        self.enter(ConnectionEnd { reason }.into())
    }

    pub(crate) fn enter(&mut self, state: ConnectionState<Pr>) -> ConnectionState<Pr> {
        self.emit(Effect::Enter(state.clone()));

        state
    }

    pub(crate) fn health(&self, state: &ConnectionState<Pr>) -> Health {
        match (self.health_check, state) {
            (Some(health_check), ConnectionState::Active(_))
                if self.missed_checks > health_check.unhealthy_threshold =>
            {
                Health::Unhealthy
            }
            _ => Health::Healthy,
        }
    }
}
