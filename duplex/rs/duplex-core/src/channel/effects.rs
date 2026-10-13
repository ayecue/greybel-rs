use super::connection::{Effect, EffectContainer};
use super::state::ChannelState;
use crate::binary::{BinaryDecode, BinaryWriter};
use crate::channel::connection::{OpenOutcome, PendingConnection};
use crate::connection_id::ConnectionId;
use crate::definition::ErrorCode;
use crate::driver::Driver;
use crate::error::Error;
use crate::frame::Control;
use crate::protocol::{Peer, Protocol};

impl<P: Peer, D: Driver> ChannelState<P, D> {
    pub(crate) fn apply(&self, effects: EffectContainer<P::Protocol>) {
        self.refresh_active();
        for effect in effects {
            match effect {
                Effect::Post {
                    control,
                    connection,
                } => self.post_control(&control, connection),
                Effect::Connection {
                    connection,
                    session,
                } => self.flush_queue(connection, &session),
                Effect::Open { id, open } => self.on_open(id, open),
                Effect::End { connection, reason } => self.on_end(connection, reason),
                Effect::Close(reason) => self.on_close(reason),
                Effect::Health(health) => self.hooks.health(health),
                Effect::Enter(state) => self.hooks.enter(&state),
            }
        }
    }

    fn on_end(&self, connection: Option<ConnectionId>, reason: ErrorCode) {
        if let (Some(connection), Some(transport)) = (connection, self.transport.upgrade()) {
            transport.release(Self::route_of(Some(connection)));
        }

        self.fragments.replace(None);
        self.deadline.take();
        self.messages
            .borrow_mut()
            .release_all(&Error::ChannelEnd(reason));
    }

    fn post_control(&self, control: &Control, connection: Option<ConnectionId>) {
        let minor = self
            .connection
            .borrow()
            .session()
            .map_or(BinaryWriter::NEWEST, |session| session.core.minor);
        let Some(transport) = self.transport.upgrade() else {
            return;
        };
        let _ = transport.write(
            Self::route_of(connection),
            self.settings.bundle,
            minor,
            usize::MAX,
            |frames| frames.control(control),
        );
    }

    fn on_close(&self, reason: ErrorCode) {
        self.health_checks.take();
        self.deadline.take();
        if let Some(transport) = self.transport.upgrade() {
            transport.flush_all();
            transport.detach(self.endpoint());
        }

        let emitters: Vec<_> = self.close_emitters.borrow_mut().drain(..).collect();

        for emitter in emitters {
            let _ = emitter.send(reason);
        }
    }

    fn on_open(&self, id: u32, open: <P::Protocol as Protocol>::Open) {
        let Some(handler) = self.handlers.connect() else {
            let outcome = if <P::Protocol as Protocol>::HANDSHAKE {
                Err(ErrorCode::UnhandledMessage)
            } else {
                <P::Protocol as Protocol>::Ack::from_bytes(&[])
                    .map_err(|_| ErrorCode::MalformedMessage)
            };

            self.on_open_result(id, outcome.map_err(Err));

            return;
        };
        let this = std::rc::Weak::clone(&self.this);

        handler(
            open,
            PendingConnection::new(move |outcome| {
                if let Some(channel) = this.upgrade() {
                    channel.on_open_result(id, outcome);
                }
            }),
        );
    }

    fn on_open_result(&self, id: u32, outcome: OpenOutcome<P::Protocol>) {
        let Some(transport) = self.transport.upgrade() else {
            return;
        };
        let random = transport.driver().random();
        let effects = self
            .connection
            .borrow_mut()
            .on_open_result(id, outcome, random);

        self.apply(effects);
    }
}
