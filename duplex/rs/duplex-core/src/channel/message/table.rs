use std::collections::VecDeque;
use std::task::{Context, Poll};
use std::time::Duration;

use slotmap::SlotMap;

use super::ids::{IdTable, LocalIds};
use super::{Body, Event, Flow, InboundState, Message, Notification, OutboundState, PendingBody};
use crate::definition::ErrorCode;
use crate::error::Error;
use crate::keys::{EntryId, NotificationId};

/// Received message registration result.
pub(crate) enum Arrival {
    New(EntryId),
    Duplicate,
    Full,
}

#[derive(Default)]
pub(crate) struct MessageTable {
    slots: SlotMap<EntryId, Message>,
    inbound: IdTable<EntryId>,
    outbound: IdTable<EntryId>,
    id_pool: LocalIds,
    queue: VecDeque<EntryId>,
    queued_count: usize,
    incoming_count: usize,
    notifications: SlotMap<NotificationId, Notification>,
}

impl MessageTable {
    pub(crate) fn get(&self, id: EntryId) -> Option<&Message> {
        self.slots.get(id)
    }

    pub(crate) fn get_mut(&mut self, id: EntryId) -> Option<&mut Message> {
        self.slots.get_mut(id)
    }

    pub(crate) fn insert(&mut self, message: Message) -> EntryId {
        self.slots.insert(message)
    }

    /// Adds a received message to the handler wait queue.
    /// Returns `Duplicate` if the exchange ID exists or `Full` if `incoming_limit` messages are waiting.
    pub(crate) fn receive(
        &mut self,
        exchange_id: u32,
        incoming_limit: usize,
        mut message: Message,
    ) -> Arrival {
        if self.inbound.get(exchange_id).is_some() {
            return Arrival::Duplicate;
        }

        if self.incoming_count >= incoming_limit {
            return Arrival::Full;
        }

        message.exchange_id = exchange_id;

        let id = self.slots.insert(message);

        self.inbound.insert(exchange_id, id);
        self.incoming_count += 1;

        Arrival::New(id)
    }

    /// Marks a ready inbound message handler-active and returns its type ID and body.
    /// Returns `None` unless the message is waiting.
    pub(crate) fn process(&mut self, id: EntryId) -> Option<(u32, Body)> {
        let message = self.slots.get_mut(id)?;
        let Flow::Inbound(inbound_state) = &mut message.flow else {
            return None;
        };

        match std::mem::replace(inbound_state, InboundState::Complete) {
            InboundState::Ready {
                connection,
                body,
                deadline,
            } => {
                *inbound_state = InboundState::HandlerActive {
                    connection,
                    deadline,
                };
                self.incoming_count -= 1;

                Some((message.type_id, body))
            }
            other => {
                *inbound_state = other;

                None
            }
        }
    }

    /// Registers a locally sent message and returns its entry ID and exchange ID.
    /// Returns `None` if `pending_limit` outbound messages are registered.
    pub(crate) fn send(
        &mut self,
        pending_limit: usize,
        mut message: Message,
    ) -> Option<(EntryId, u32)> {
        if self.outbound.len() >= pending_limit {
            return None;
        }

        let exchange_id = self.id_pool.take();

        message.exchange_id = exchange_id;

        let id = self.slots.insert(message);

        self.outbound.insert(exchange_id, id);

        Some((id, exchange_id))
    }

    pub(crate) fn push_notification(&mut self, notification: Notification) -> NotificationId {
        self.notifications.insert(notification)
    }

    pub(crate) fn take_notification(&mut self, id: NotificationId) -> Option<Notification> {
        self.notifications.remove(id)
    }

    pub(crate) fn inbound_id(&self, exchange_id: u32) -> Option<EntryId> {
        self.inbound.get(exchange_id).copied()
    }

    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn outbound_id(&self, exchange_id: u32) -> Option<EntryId> {
        self.outbound.get(exchange_id).copied()
    }

    pub(crate) fn active_response_count(&self) -> usize {
        self.inbound.len() - self.incoming_count
    }

    pub(crate) fn outbound_count(&self) -> usize {
        self.outbound.len()
    }

    pub(crate) fn queued_count(&self) -> usize {
        self.queued_count
    }

    pub(crate) fn queue(&mut self, id: EntryId) {
        self.queue.push_back(id);
        self.queued_count += 1;
    }

    pub(crate) fn decrease_queue(&mut self) {
        self.queued_count = self.queued_count.saturating_sub(1);
    }

    pub(crate) fn take_next(&mut self) -> VecDeque<EntryId> {
        self.queued_count = 0;

        std::mem::take(&mut self.queue)
    }

    pub(crate) fn dequeue(&mut self, id: EntryId) -> Option<(u32, Option<u32>, PendingBody)> {
        let message = self.slots.get_mut(id)?;

        if let Flow::Outbound(outbound_state) = &mut message.flow {
            let pending_body = outbound_state.dequeue()?;

            return Some((message.type_id, Some(message.exchange_id), pending_body));
        }

        if !matches!(message.flow, Flow::Notification(_)) {
            return None;
        }

        let message = self.slots.remove(id)?;
        let Flow::Notification(pending_body) = message.flow else {
            return None;
        };

        Some((message.type_id, None, pending_body))
    }

    pub(crate) fn release_from_outbound(&mut self, id: EntryId) {
        let Some(exchange_id) = self
            .slots
            .get(id)
            .filter(|message| message.is_outbound())
            .map(|message| message.exchange_id)
        else {
            return;
        };

        if self
            .outbound
            .remove_if(exchange_id, |&owner| owner == id)
            .is_some()
        {
            self.id_pool.release(exchange_id);
        }
    }

    pub(crate) fn try_finish(&mut self, id: EntryId, error: &Error, release: bool) {
        let Some(message) = self.slots.get_mut(id) else {
            return;
        };
        let exchange_id = message.exchange_id;

        if message.is_ready() {
            self.incoming_count -= 1;
        }

        message.fail(error);
        if release {
            message.refs = message.refs.saturating_sub(1);
        }

        let removable = message.is_removable();

        self.inbound.remove_if(exchange_id, |&owner| owner == id);
        if removable {
            self.remove(id);
        }
    }

    pub(crate) fn handle_event(&mut self, id: EntryId, event: Event, stream_error: &Error) {
        let Some(message) = self.slots.get_mut(id) else {
            return;
        };

        message.on_event(event, stream_error);
        self.finish_outbound(id);
    }

    pub(crate) fn receive_response(
        &mut self,
        id: EntryId,
        event: Event,
        stream_error: &Error,
    ) -> bool {
        self.slots
            .get_mut(id)
            .is_some_and(|message| message.receive_response(event, stream_error))
    }

    pub(crate) fn defer_response_item(&mut self, id: EntryId, event: Event) -> Result<(), Event> {
        let Some(message) = self.slots.get_mut(id) else {
            return Err(event);
        };

        message.defer_response_item(event)
    }

    pub(crate) fn deliver_response_item(&mut self, id: EntryId) -> Option<bool> {
        self.slots.get_mut(id)?.deliver_response_item()
    }

    pub(crate) fn settle_response(&mut self, id: EntryId) {
        let Some(message) = self.slots.get_mut(id) else {
            return;
        };

        message.deliver_response();
        self.finish_outbound(id);
    }

    fn finish_outbound(&mut self, id: EntryId) {
        let Some(message) = self.slots.get_mut(id) else {
            return;
        };
        let exchange_id = message.is_outbound().then_some(message.exchange_id);
        let removable = message.is_removable();

        if let Some(exchange_id) = exchange_id
            && self
                .outbound
                .remove_if(exchange_id, |&owner| owner == id)
                .is_some()
        {
            self.id_pool.release(exchange_id);
        }

        if removable {
            self.remove(id);
        }
    }

    pub(crate) fn poll_outcome(&mut self, id: EntryId, context: &mut Context<'_>) -> Poll<Event> {
        let Some(message) = self.slots.get_mut(id) else {
            return Poll::Ready(Event::Error(Error::ChannelEnd(ErrorCode::ClosedChannel)));
        };
        let outcome = message.poll_outcome(context);

        if outcome.is_ready() {
            message.refs = message.refs.saturating_sub(1);
            if message.is_removable() {
                self.remove(id);
            }
        }

        outcome
    }

    pub(crate) fn try_remove(&mut self, id: EntryId) {
        if self.slots.get(id).is_some_and(Message::is_removable) {
            self.remove(id);
        }
    }

    pub(crate) fn remove(&mut self, id: EntryId) -> Option<Message> {
        self.slots.remove(id)
    }

    pub(crate) fn add_refs(&mut self, id: EntryId, refs: u8) -> bool {
        let Some(message) = self.slots.get_mut(id) else {
            return false;
        };

        message.refs += refs;

        message
            .streams
            .as_ref()
            .is_some_and(|streams| streams.input.is_some())
    }

    pub(crate) fn release(&mut self, id: EntryId) {
        self.release_by(id, 1);
    }

    pub(crate) fn release_by(&mut self, id: EntryId, refs: u8) {
        let Some(message) = self.slots.get_mut(id) else {
            return;
        };

        message.refs = message.refs.saturating_sub(refs);
        if message.is_removable() {
            self.remove(id);
        }
    }

    pub(crate) fn due(&self, now: Duration) -> Vec<EntryId> {
        self.inbound
            .iter()
            .map(|(_, &id)| id)
            .filter(|&id| {
                self.slots
                    .get(id)
                    .and_then(Message::deadline)
                    .is_some_and(|deadline| deadline <= now)
            })
            .collect()
    }

    pub(crate) fn next_deadline(&self) -> Option<Duration> {
        self.inbound
            .iter()
            .filter_map(|(_, &id)| self.slots.get(id)?.deadline())
            .min()
    }

    pub(crate) fn release_all(&mut self, reason: &Error) {
        self.notifications.clear();
        for (_, message) in &mut self.slots {
            match message.flow {
                Flow::Outbound(OutboundState::Outcome(_)) | Flow::Notification(_) => {}
                Flow::Outbound(OutboundState::CancelPending(_)) => {
                    message.on_event(Event::Error(reason.clone()), reason);
                }
                Flow::Outbound(_) => message.fail_stream_and_emit(reason, false),
                Flow::Inbound(_) if message.is_active() => message.fail(reason),
                Flow::Inbound(_) => message.flow = Flow::Inbound(InboundState::Complete),
            }
        }

        self.inbound.drain();

        let mut message_ids: Vec<u32> = self
            .outbound
            .iter()
            .map(|(exchange_id, _)| exchange_id)
            .collect();

        message_ids.sort_unstable();
        self.outbound.drain();
        for exchange_id in message_ids {
            self.id_pool.release(exchange_id);
        }

        self.queue.clear();
        self.queued_count = 0;
        self.incoming_count = 0;
        self.slots.retain(|_, message| {
            !message.is_removable() && !matches!(message.flow, Flow::Notification(_))
        });
    }
}
