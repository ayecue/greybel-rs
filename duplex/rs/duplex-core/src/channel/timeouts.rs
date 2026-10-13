use std::rc::Weak;
use std::time::Duration;

use scheduler::{Clock, Scheduler};

use super::message::{Message, OutputHandle};
use super::state::ChannelState;
use crate::definition::ErrorCode;
use crate::driver::Driver;
use crate::protocol::Peer;

impl<P: Peer, D: Driver> ChannelState<P, D> {
    /// Resets received message `message` deadline from its configured timeout.
    pub(super) fn refresh_deadline_of(&self, message: &mut Message) {
        let Some(timeout) = self.timeout_of(message.type_id) else {
            return;
        };

        if let Some(now) = self.now() {
            message.set_deadline(now + timeout);
        }
    }

    /// Returns the declared message timeout or the channel default.
    pub(super) fn timeout_of(&self, type_id: u32) -> Option<Duration> {
        <P::Remote as Peer>::TIMEOUTS
            .get(type_id as usize)
            .copied()
            .flatten()
            .or(self.settings.timeout)
    }

    pub(super) fn schedule_deadline(&self, at: Duration) {
        if self
            .deadline
            .borrow()
            .as_ref()
            .is_some_and(|(due_at, _)| *due_at <= at)
        {
            return;
        }

        let Some(transport) = self.transport.upgrade() else {
            return;
        };
        let driver = transport.driver();
        let this = Weak::clone(&self.this);
        let timer = driver.scheduler().after(
            at.saturating_sub(driver.scheduler().now()),
            Box::new(move || {
                if let Some(channel) = this.upgrade() {
                    channel.on_deadline();
                }
            }),
        );

        self.deadline.replace(Some((at, timer)));
    }

    fn on_deadline(&self) {
        self.deadline.take();

        let Some(now) = self.now() else {
            return;
        };
        let due = self.messages.borrow().due(now);

        for id in due {
            let waits = {
                let mut messages = self.messages.borrow_mut();
                let Some(message) = messages.get_mut(id) else {
                    continue;
                };
                let waits = message
                    .streams
                    .as_ref()
                    .and_then(|streams| streams.output.as_ref())
                    .is_some_and(OutputHandle::is_pending);

                if waits && let Some(timeout) = self.timeout_of(message.type_id) {
                    message.set_deadline(now + timeout);
                }

                waits
            };

            if !waits {
                self.reject_inbound(id, ErrorCode::Timeout);
            }
        }

        let next = self.messages.borrow().next_deadline();

        if let Some(next) = next {
            self.schedule_deadline(next);
        }
    }
}
