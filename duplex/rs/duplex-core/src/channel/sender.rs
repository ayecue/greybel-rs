use std::task::{Context, Poll, ready};

use super::SendOptions;
use super::exchange::{ItemSource, OutgoingExchange};
use super::message::{
    Event, Flow, INITIAL_CREDIT, MESSAGE_STREAM, Message, OutboundState, PendingBody,
    ReceiveWindow, StreamPair,
};
use super::state::{ChannelState, FrameTo};
use crate::binary::{ValidatedEncode, Validation};
use crate::connection_id::ConnectionId;
use crate::definition::{ErrorCode, Requirement, Violation};
use crate::driver::Driver;
use crate::error::Error;
use crate::keys::EntryId;
use crate::migration::{MessageWrite, MessageWriteBody, SegmentWrite, Session};
use crate::protocol::{Exchange, MessageRequirementTable, MessageSegment, Peer};

type HandleOf<M> = <<M as crate::protocol::Message>::Exchange as Exchange>::Handle;

struct OutputTarget {
    exchange_id: u32,
    type_id: u32,
    connection: ConnectionId,
    requirements: &'static MessageRequirementTable,
    item_count: u32,
}

impl<P: Peer, D: Driver> ChannelState<P, D> {
    pub(crate) fn send<M: crate::protocol::Message<Sender = P>>(
        &self,
        body: &M,
        options: SendOptions,
    ) -> Result<HandleOf<M>, Error> {
        let bundle = options.bundle.unwrap_or(self.settings.bundle);

        if self.settings.validating {
            body.validate(&Validation::Any)?;
        }

        let route = self.active_target()?;

        if !M::Exchange::RESPONDS {
            match route {
                Some((connection, session)) => {
                    let limit = self.limit_of(M::REQUIREMENTS);
                    let message = MessageWrite {
                        type_id: M::ID,
                        id: None,
                        since: M::SINCE,
                        body: MessageWriteBody::Value(body),
                    };

                    self.write_frame(bundle, Some(connection), session.minor(), |frames| {
                        frames.set_limit(limit);

                        session.write_message(frames, &message)
                    })?;
                }
                None => {
                    let pending_body = self.encode_body::<M>(body, bundle)?;
                    let message = Message::new(M::ID, Flow::Notification(pending_body), None);
                    let mut messages = self.messages.borrow_mut();
                    let id = messages.insert(message);

                    messages.queue(id);
                }
            }

            return Ok(M::Exchange::handle(None));
        }

        let limit = self.settings.limits.pending_items;

        if route.is_none() && self.messages.borrow().outbound_count() >= limit {
            return Err(Error::Rejection(ErrorCode::Overload));
        }

        let initial_state = match &route {
            Some(_) => OutboundState::Unsent(None),
            None => OutboundState::Pending(self.encode_body::<M>(body, bundle)?, None),
        };
        let input = M::Exchange::STREAMS_RESPONSE
            .then(|| ReceiveWindow::new(self.settings.window, M::REQUIREMENTS.response_item_size));
        let output = M::Exchange::STREAMS.then_some(0);
        let streams = StreamPair::of(M::ID, input, output);
        let mut message = Message::new(M::ID, Flow::Outbound(initial_state), streams);

        message.refs = M::Exchange::SENT_REFS;

        let (id, exchange_id) = {
            let mut messages = self.messages.borrow_mut();
            let ids = messages
                .send(limit, message)
                .ok_or(Error::Rejection(ErrorCode::Overload))?;

            if route.is_none() {
                messages.queue(ids.0);
            }

            ids
        };

        if let Some((connection, session)) = route {
            let message = MessageWrite {
                type_id: M::ID,
                id: Some(exchange_id),
                since: M::SINCE,
                body: MessageWriteBody::Value(body),
            };
            let limit = self.limit_of(M::REQUIREMENTS);
            let to = FrameTo {
                connection,
                bundle,
                minor: session.minor(),
                starts: true,
            };

            if let Err(error) = self.write_message_frame(id, &to, |frames| {
                frames.set_limit(limit);

                session.write_message(frames, &message)
            }) {
                let mut messages = self.messages.borrow_mut();

                messages.release_from_outbound(id);
                messages.remove(id);

                return Err(error);
            }
        }

        Ok(M::Exchange::handle(Some(OutgoingExchange {
            channel: self.exchange_channel(),
            message: id,
        })))
    }

    fn encode_body<M: crate::protocol::Message>(
        &self,
        body: &M,
        bundle: super::Bundle,
    ) -> Result<PendingBody, Error> {
        if self.messages.borrow().queued_count() >= self.settings.limits.queued_items {
            return Err(Error::Rejection(ErrorCode::Overload));
        }

        let bytes = body.to_bytes()?;

        if bytes.len() > self.limit_of(M::REQUIREMENTS) {
            return Err(Error::Invalid(Violation {
                path: Vec::new(),
                requirement: Requirement::Limit,
            }));
        }

        Ok(PendingBody {
            body: bytes,
            since: M::SINCE,
            transcode: M::transcode,
            bundle,
        })
    }

    pub(crate) fn on_dispatch(&self, id: EntryId) {
        let reads = {
            let mut messages = self.messages.borrow_mut();
            let Some(message) = messages.get_mut(id) else {
                return;
            };
            let Flow::Outbound(outbound) = &mut message.flow else {
                return;
            };

            if !outbound.finish_dispatch() {
                return;
            }

            if let Some(output) = message.output() {
                output.grant(INITIAL_CREDIT);
            }

            message.input().is_some()
        };

        if reads {
            self.start_input(id);
        }
    }

    pub(crate) fn flush_queue(&self, connection: ConnectionId, session: &Session) {
        let queued_ids = self.messages.borrow_mut().take_next();

        for id in queued_ids {
            let Some((type_id, exchange_id, pending_body)) = self.messages.borrow_mut().dequeue(id)
            else {
                continue;
            };
            let write = MessageWrite {
                type_id,
                id: exchange_id,
                since: pending_body.since,
                body: MessageWriteBody::Pending(&pending_body, Self::NEWEST),
            };

            if exchange_id.is_none() {
                let _ = self.write_frame(
                    pending_body.bundle,
                    Some(connection),
                    session.minor(),
                    |frames| session.write_message(frames, &write),
                );

                continue;
            }

            let to = FrameTo {
                connection,
                bundle: pending_body.bundle,
                minor: session.minor(),
                starts: true,
            };

            if let Err(error) =
                self.write_message_frame(id, &to, |frames| session.write_message(frames, &write))
            {
                let mut messages = self.messages.borrow_mut();

                messages.release_from_outbound(id);
                if let Some(message) = messages.get_mut(id) {
                    message.fail_stream_and_emit(&error, false);
                }

                messages.try_remove(id);
            }
        }
    }

    pub(crate) fn release_on_settle(&self, id: EntryId, event: Event, stream_error: &Error) {
        self.messages
            .borrow_mut()
            .handle_event(id, event, stream_error);
    }

    pub(crate) fn cancel_outbound(&self, id: EntryId, code: ErrorCode) {
        let error = Error::Rejection(code);
        let state = self
            .messages
            .borrow()
            .get(id)
            .map(|message| match message.flow {
                Flow::Outbound(OutboundState::Pending(..)) => Some(false),
                Flow::Outbound(OutboundState::Unsent(_) | OutboundState::ResponsePending(_)) => {
                    Some(true)
                }
                _ => None,
            });
        let Some(Some(in_flight)) = state else {
            return;
        };
        let route = if in_flight {
            self.active_target().ok().flatten()
        } else {
            None
        };
        let exchange_id = {
            let mut messages = self.messages.borrow_mut();
            let Some(message) = messages.get_mut(id) else {
                return;
            };

            message.fail_stream_and_emit(&error, route.is_some());

            let exchange_id = message.exchange_id;

            if !in_flight {
                messages.decrease_queue();
            }

            if route.is_none() {
                messages.release_from_outbound(id);
                messages.try_remove(id);
            }

            exchange_id
        };

        if let Some((connection, session)) = route {
            let to = FrameTo {
                connection,
                bundle: self.settings.bundle,
                minor: session.minor(),
                starts: false,
            };
            let _ = self.write_message_frame(id, &to, |frames| frames.cancel(exchange_id));
        }
    }

    /// Resolves stream destination and session for message `id`.
    /// Waits for and consumes one stream-credit item when a context is supplied.
    fn output_target(
        &self,
        id: EntryId,
        context: Option<&mut Context<'_>>,
    ) -> Poll<Result<(OutputTarget, Session), Error>> {
        let target = {
            let mut messages = self.messages.borrow_mut();
            let message = messages
                .get_mut(id)
                .ok_or(Error::ChannelEnd(ErrorCode::ClosedChannel))?;

            if let Some(context) = context {
                let output = message
                    .output()
                    .ok_or(Error::ChannelEnd(ErrorCode::ClosedChannel))?;

                ready!(output.poll_take(context))?;
            }

            match message.flow {
                Flow::Outbound(OutboundState::Unsent(_) | OutboundState::ResponsePending(_)) => {}
                Flow::Outbound(OutboundState::CancelPending(_)) => {
                    return Poll::Ready(Err(Error::Rejection(ErrorCode::Cancellation)));
                }
                _ => return Poll::Ready(Err(Error::ChannelEnd(ErrorCode::ClosedChannel))),
            }

            (
                message.exchange_id,
                message.type_id,
                Self::outbound_requirements(message.type_id),
                message.items(MESSAGE_STREAM),
            )
        };
        let Some((connection, session)) = self.active_target()? else {
            return Poll::Ready(Err(Error::ChannelEnd(ErrorCode::ConnectionLoss)));
        };
        let (exchange_id, type_id, requirements, item_count) = target;

        Poll::Ready(Ok((
            OutputTarget {
                exchange_id,
                type_id,
                connection,
                requirements,
                item_count,
            },
            session,
        )))
    }

    pub(crate) fn write_message_stream(
        &self,
        id: EntryId,
        segment: MessageSegment,
        value: &dyn ValidatedEncode,
    ) -> Result<(), Error> {
        match self.write_stream_segment(id, segment, value, None) {
            Poll::Ready(result) => result,
            Poll::Pending => unreachable!("only taking credit waits"),
        }
    }

    /// Writes the next stream item for message `id` when credit is available.
    pub(crate) fn write_message_item(
        &self,
        id: EntryId,
        value: &dyn ValidatedEncode,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        self.write_stream_segment(id, MessageSegment::MessageItem, value, Some(context))
    }

    fn write_stream_segment(
        &self,
        id: EntryId,
        segment: MessageSegment,
        value: &dyn ValidatedEncode,
        context: Option<&mut Context<'_>>,
    ) -> Poll<Result<(), Error>> {
        let (target, session) = ready!(self.output_target(id, context))?;

        self.validate_segment(target.requirements, segment, value, target.item_count)?;

        let write = SegmentWrite {
            segment,
            id: target.exchange_id,
            type_id: target.type_id,
            value,
        };
        let limit = self.limit_of(target.requirements);
        let to = FrameTo {
            connection: target.connection,
            bundle: self.settings.bundle,
            minor: session.minor(),
            starts: false,
        };
        let result = self.write_message_frame(id, &to, |frames| {
            frames.set_limit(limit);

            session.write_segment(frames, &write)
        });

        match &result {
            Ok(()) if segment == MessageSegment::MessageItem => {
                if let Some(message) = self.messages.borrow_mut().get_mut(id) {
                    message.count_item(MESSAGE_STREAM);
                }
            }
            Err(Error::Invalid(_)) | Ok(()) => {}
            Err(_) => self.cancel_outbound(id, ErrorCode::Cancellation),
        }

        Poll::Ready(result)
    }

    pub(crate) fn write_message_items(
        &self,
        id: EntryId,
        items: &mut dyn ItemSource,
    ) -> Result<(), Error> {
        let (target, session) = match self.output_target(id, None) {
            Poll::Ready(target) => target?,
            Poll::Pending => unreachable!("only taking credit waits"),
        };
        let mut credit = self.take_all_credit(id)?;
        let segment = MessageSegment::MessageItem;
        let limit = self.limit_of(target.requirements);
        let to = FrameTo {
            connection: target.connection,
            bundle: self.settings.bundle,
            minor: session.minor(),
            starts: false,
        };
        let mut item_count = target.item_count;
        let mut result = Ok(());

        while credit > 0 {
            let next = items.write_next(&mut |value| {
                self.validate_segment(target.requirements, segment, value, item_count)?;

                let write = SegmentWrite {
                    segment,
                    id: target.exchange_id,
                    type_id: target.type_id,
                    value,
                };

                self.write_message_frame(id, &to, |frames| {
                    frames.set_limit(limit);

                    session.write_segment(frames, &write)
                })
            });

            match next {
                None => break,
                Some(Err(error)) => {
                    result = Err(error);
                    break;
                }
                Some(Ok(())) => {
                    credit -= 1;
                    item_count += 1;
                }
            }
        }

        if let Some(streams) = self
            .messages
            .borrow_mut()
            .get_mut(id)
            .and_then(|message| message.streams.as_mut())
        {
            if let Some(output) = &mut streams.output {
                output.release(credit);
            }

            streams.items[MESSAGE_STREAM] = item_count;
        }

        if let Err(error) = &result
            && !matches!(error, Error::Invalid(_))
        {
            self.cancel_outbound(id, ErrorCode::Cancellation);
        }

        result
    }

    pub(crate) fn take_all_credit(&self, id: EntryId) -> Result<u32, Error> {
        self.messages
            .borrow_mut()
            .get_mut(id)
            .and_then(Message::output)
            .ok_or(Error::ChannelEnd(ErrorCode::ClosedChannel))?
            .take_all()
    }
}
