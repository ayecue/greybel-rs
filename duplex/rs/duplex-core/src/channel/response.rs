use std::task::{Context, Poll, ready};

use super::exchange::{IncomingExchange, ItemSource};
use super::message::RESPONSE_STREAM;
use super::state::{ChannelState, FrameTo};
use crate::binary::ValidatedEncode;
use crate::connection_id::ConnectionId;
use crate::definition::ErrorCode;
use crate::driver::Driver;
use crate::error::Error;
use crate::frame::Rejection;
use crate::keys::EntryId;
use crate::migration::{SegmentWrite, Session};
use crate::protocol::{MessageRequirementTable, MessageSegment, Peer};

const NO_REFRESH: bool = false;

struct ResponseTarget {
    exchange_id: u32,
    type_id: u32,
    connection: ConnectionId,
    requirements: &'static MessageRequirementTable,
    item_count: u32,
}

impl<P: Peer, D: Driver> ChannelState<P, D> {
    /// Creates a handler-side exchange for received message `id`.
    /// Requires pre-counted message references; `reads` sets initial input-stream credit.
    pub(crate) fn start_exchange(&self, id: EntryId, reads: bool) -> IncomingExchange {
        if reads {
            self.start_input(id);
        }

        IncomingExchange {
            channel: self.exchange_channel(),
            message: id,
        }
    }

    /// Resolves response destination and session for message `id`.
    /// Optionally refreshes the deadline, then waits for one stream-credit item.
    fn response_target(
        &self,
        id: EntryId,
        refresh: bool,
        context: Option<&mut Context<'_>>,
    ) -> Poll<Result<(ResponseTarget, Session), Error>> {
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

            if refresh {
                self.refresh_deadline_of(message);
            }

            ResponseTarget {
                exchange_id: message.exchange_id,
                type_id: message.type_id,
                connection: message
                    .connection()
                    .ok_or(Error::ChannelEnd(ErrorCode::ClosedChannel))?,
                requirements: Self::inbound_requirements(message.type_id),
                item_count: message.items(RESPONSE_STREAM),
            }
        };
        let session = self
            .session_of(target.connection)
            .ok_or(Error::ChannelEnd(ErrorCode::ConnectionLoss))?;

        Poll::Ready(Ok((target, session)))
    }

    fn response_to(&self, target: &ResponseTarget, session: &Session) -> FrameTo {
        FrameTo {
            connection: target.connection,
            bundle: self.settings.bundle,
            minor: session.minor(),
            starts: false,
        }
    }

    pub(crate) fn write_response(
        &self,
        id: EntryId,
        segment: MessageSegment,
        value: &dyn ValidatedEncode,
    ) -> Result<(), Error> {
        self.write_response_and_release(id, segment, value, false)
    }

    /// Writes response `segment` for message `id`.
    /// Releases one message reference after the operation when `release` is true, including on error.
    pub(crate) fn write_response_and_release(
        &self,
        id: EntryId,
        segment: MessageSegment,
        value: &dyn ValidatedEncode,
        release: bool,
    ) -> Result<(), Error> {
        match self.write_response_segment(id, segment, value, release, None) {
            Poll::Ready(result) => result,
            Poll::Pending => unreachable!("only taking credit waits"),
        }
    }

    /// Writes the next response item for message `id` when credit is available.
    pub(crate) fn write_response_item(
        &self,
        id: EntryId,
        value: &dyn ValidatedEncode,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        self.write_response_segment(
            id,
            MessageSegment::ResponseItem,
            value,
            false,
            Some(context),
        )
    }

    fn write_response_segment(
        &self,
        id: EntryId,
        segment: MessageSegment,
        value: &dyn ValidatedEncode,
        release: bool,
        context: Option<&mut Context<'_>>,
    ) -> Poll<Result<(), Error>> {
        let ends = segment.is_response_end();
        let (target, session) = match ready!(self.response_target(id, !ends, context)) {
            Ok(target) => target,
            Err(error) => {
                if release {
                    self.messages.borrow_mut().release(id);
                }

                return Poll::Ready(Err(error));
            }
        };
        let write = SegmentWrite {
            segment,
            id: target.exchange_id,
            type_id: target.type_id,
            value,
        };
        let limit = self.limit_of(target.requirements);
        let to = self.response_to(&target, &session);
        let result = self
            .validate_segment(target.requirements, segment, value, target.item_count)
            .and_then(|()| {
                self.write_message_frame(id, &to, |frames| {
                    frames.set_limit(limit);

                    session.write_segment(frames, &write)
                })
            });

        match &result {
            Ok(()) if ends => {
                self.try_finish(id, &Error::ChannelEnd(ErrorCode::ClosedChannel), release);

                return Poll::Ready(result);
            }
            Ok(()) if segment == MessageSegment::ResponseItem => {
                if let Some(message) = self.messages.borrow_mut().get_mut(id) {
                    message.count_item(RESPONSE_STREAM);
                }
            }
            Ok(()) => {}
            Err(error) => self.reject_inbound(id, Rejection::of(error)),
        }

        if release {
            self.messages.borrow_mut().release(id);
        }

        Poll::Ready(result)
    }

    pub(crate) fn write_response_items(
        &self,
        id: EntryId,
        items: &mut dyn ItemSource,
    ) -> Result<(), Error> {
        let (target, session) = match self.response_target(id, NO_REFRESH, None) {
            Poll::Ready(target) => target?,
            Poll::Pending => unreachable!("only taking credit waits"),
        };
        let mut credit = self.take_all_credit(id)?;
        let segment = MessageSegment::ResponseItem;
        let limit = self.limit_of(target.requirements);
        let to = self.response_to(&target, &session);
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

        if let Some(message) = self.messages.borrow_mut().get_mut(id) {
            if let Some(streams) = message.streams.as_mut() {
                if let Some(output) = &mut streams.output {
                    output.release(credit);
                }

                streams.items[RESPONSE_STREAM] = item_count;
            }

            if item_count > target.item_count {
                self.refresh_deadline_of(message);
            }
        }

        if let Err(error) = &result {
            self.reject_inbound(id, Rejection::of(error));
        }

        result
    }

    pub(crate) fn reject_inbound(&self, id: EntryId, rejection: impl Into<Rejection>) {
        let rejection = rejection.into();
        let error = rejection.error();
        let target = {
            let mut messages = self.messages.borrow_mut();
            let Some(message) = messages.get_mut(id).filter(|message| message.is_active()) else {
                return;
            };

            message.fail_stream(&error);

            message
                .connection()
                .map(|connection| (connection, message.exchange_id))
        };

        if let Some((connection, exchange_id)) = target
            && let Some(session) = self.session_of(connection)
        {
            let to = FrameTo {
                connection,
                bundle: self.settings.bundle,
                minor: session.minor(),
                starts: false,
            };
            let _ = self
                .write_message_frame(id, &to, |frames| frames.rejection(exchange_id, &rejection));
        }

        self.try_finish(id, &error, false);
    }

    fn try_finish(&self, id: EntryId, error: &Error, release: bool) {
        self.messages.borrow_mut().try_finish(id, error, release);
    }

    pub(crate) fn write_rejection(
        &self,
        connection: ConnectionId,
        exchange_id: u32,
        rejection: impl Into<Rejection>,
    ) {
        let Some(session) = self.session_of(connection) else {
            return;
        };
        let rejection = rejection.into();
        let _ = self.write_frame(
            self.settings.bundle,
            Some(connection),
            session.minor(),
            |frames| frames.rejection(exchange_id, &rejection),
        );
    }
}
