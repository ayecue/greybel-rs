use std::task::{Context, Poll};

use super::exchange::{ChannelIo, ItemSource};
use super::message::{Event, Flow, Message, OutboundState, OutputHandle};
use super::state::ChannelState;
use crate::binary::ValidatedEncode;
use crate::definition::{ErrorCode, FrameKind};
use crate::driver::Driver;
use crate::error::Error;
use crate::frame::Rejection;
use crate::keys::EntryId;
use crate::protocol::{MessageSegment, Peer};

fn closed_error() -> Error {
    Error::ChannelEnd(ErrorCode::ClosedChannel)
}

impl<P: Peer, D: Driver> ChannelState<P, D> {
    fn poll_output(
        &self,
        id: EntryId,
        poll: impl FnOnce(&mut OutputHandle) -> Poll<Result<(), Error>>,
    ) -> Poll<Result<(), Error>> {
        match self
            .messages
            .borrow_mut()
            .get_mut(id)
            .and_then(Message::output)
        {
            Some(output) => poll(output),
            None => Poll::Ready(Err(closed_error())),
        }
    }

    pub(crate) fn start_input(&self, id: EntryId) {
        let count = self
            .messages
            .borrow_mut()
            .get_mut(id)
            .and_then(|message| message.input()?.start());

        if let Some(count) = count {
            self.write_credit(id, count);
        }
    }

    fn write_credit(&self, id: EntryId, count: u32) {
        let target = {
            let messages = self.messages.borrow();
            let Some(message) = messages.get(id) else {
                return;
            };

            match message.flow {
                Flow::Outbound(OutboundState::ResponsePending(_)) => self
                    .current_connection()
                    .map(|connection| (FrameKind::ResponseCredit, connection, message.exchange_id)),
                _ => message
                    .connection()
                    .map(|connection| (FrameKind::MessageCredit, connection, message.exchange_id)),
            }
        };
        let Some((kind, connection, exchange_id)) = target else {
            return;
        };
        let Some(session) = self.session_of(connection) else {
            return;
        };
        let _ = self.write_frame(
            self.settings.bundle,
            Some(connection),
            session.minor(),
            |frames| frames.credit(kind, exchange_id, count),
        );
    }

    /// Returns whether message `id` is outbound, or `None` if unregistered.
    fn is_outbound(&self, id: EntryId) -> Option<bool> {
        let messages = self.messages.borrow();
        let message = messages.get(id)?;

        Some(message.is_outbound())
    }
}

impl<P: Peer, D: Driver> ChannelIo for ChannelState<P, D> {
    fn release(&self, id: EntryId) {
        self.messages.borrow_mut().release(id);
    }

    fn release_input(&self, id: EntryId) {
        let mut messages = self.messages.borrow_mut();

        if let Some(streams) = messages
            .get_mut(id)
            .and_then(|message| message.streams.as_mut())
        {
            streams.input = None;
        }

        messages.release(id);
    }

    fn release_output(&self, id: EntryId) {
        let mut messages = self.messages.borrow_mut();

        if let Some(streams) = messages
            .get_mut(id)
            .and_then(|message| message.streams.as_mut())
        {
            streams.output = None;
        }

        messages.release(id);
    }

    fn poll_outcome(&self, id: EntryId, context: &mut Context<'_>) -> Poll<Event> {
        self.messages.borrow_mut().poll_outcome(id, context)
    }

    fn cancel_message(&self, id: EntryId) {
        self.cancel_outbound(id, ErrorCode::Cancellation);
    }

    fn poll_event(&self, id: EntryId, context: &mut Context<'_>) -> Poll<Event> {
        let (event, credit) = {
            let mut messages = self.messages.borrow_mut();
            let Some(input) = messages.get_mut(id).and_then(Message::input) else {
                return Poll::Ready(Event::Error(closed_error()));
            };
            let Poll::Ready(event) = input.poll(context) else {
                return Poll::Pending;
            };
            let credit = match event {
                Event::Item(_) => input.consume_item(),
                _ => None,
            };

            (event, credit)
        };

        if let Some(count) = credit {
            self.write_credit(id, count);
        }

        Poll::Ready(event)
    }

    fn poll_stream_item(
        &self,
        id: EntryId,
        value: &dyn ValidatedEncode,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        self.write_message_item(id, value, context)
    }

    fn poll_response_item(
        &self,
        id: EntryId,
        value: &dyn ValidatedEncode,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        self.write_response_item(id, value, context)
    }

    fn poll_available(&self, id: EntryId, context: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.poll_output(id, |output| output.poll_available(context))
    }

    fn poll_open(&self, id: EntryId, context: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.poll_output(id, |output| output.poll_open(context))
    }

    fn fail_input(&self, id: EntryId, rejection: Rejection) {
        match self.is_outbound(id) {
            Some(true) => self.cancel_outbound(id, ErrorCode::Cancellation),
            Some(false) => self.reject_inbound(id, rejection),
            _ => {}
        }
    }

    fn write_stream(
        &self,
        id: EntryId,
        segment: MessageSegment,
        value: &dyn ValidatedEncode,
    ) -> Result<(), Error> {
        self.write_message_stream(id, segment, value)
    }

    fn write_items(&self, id: EntryId, items: &mut dyn ItemSource) -> Result<(), Error> {
        match self.is_outbound(id) {
            Some(true) => self.write_message_items(id, items),
            Some(false) => self.write_response_items(id, items),
            _ => Err(closed_error()),
        }
    }

    fn respond(
        &self,
        id: EntryId,
        segment: MessageSegment,
        value: &dyn ValidatedEncode,
    ) -> Result<(), Error> {
        self.write_response(id, segment, value)
    }

    fn settle(&self, id: EntryId, segment: MessageSegment, value: &dyn ValidatedEncode) {
        let _ = self.write_response_and_release(id, segment, value, true);
    }

    fn reject(&self, id: EntryId, rejection: Rejection) {
        self.reject_inbound(id, rejection);
        self.messages.borrow_mut().release(id);
    }

    fn is_finished(&self, id: EntryId) -> bool {
        !self
            .messages
            .borrow()
            .get(id)
            .is_some_and(Message::is_active)
    }
}
