use std::rc::Rc;

use super::message::{
    Arrival, Body, Event, Flow, INITIAL_CREDIT, InboundState, MESSAGE_STREAM, Message,
    Notification, OutboundState, RESPONSE_STREAM, ReceiveWindow, StreamPair,
};
use super::router::Streaming;
use super::state::ChannelState;
use crate::binary::Validation;
use crate::connection_id::ConnectionId;
use crate::definition::ErrorCode;
use crate::driver::Driver;
use crate::error::Error;
use crate::frame::{CORE_MAJOR, Frame, FrameReader, Rejection};
use crate::keys::{EntryId, NotificationId, Work};
use crate::migration::Session;
use crate::protocol::{MessageRequirementTable, MessageSegment, Peer};
use crate::transport::Inbound;

struct FrameTarget {
    id: EntryId,
    type_id: u32,
    requirements: &'static MessageRequirementTable,
    item_count: u32,
}

impl<P: Peer, D: Driver> ChannelState<P, D> {
    pub(crate) fn read_slice(self: &Rc<Self>, inbound: Inbound<'_>) {
        let matches_core_major = inbound.core == CORE_MAJOR;
        let mut active = false;
        let mut frames = FrameReader::new(inbound.batch, inbound.frames);

        while let Some(frame) = frames.next() {
            let Ok(frame) = frame else {
                return;
            };

            if let Frame::Fragment { ends, bytes } = frame {
                self.on_fragment(
                    inbound.connection,
                    inbound.core,
                    &inbound.batch[bytes],
                    ends,
                );
                continue;
            }

            if frames.last_length() > self.settings.max_message
                && !matches!(frame, Frame::Control(_))
            {
                let (kind, id) = frame.kind_and_id();

                self.refuse_over_limit(inbound.connection, kind, id);
                continue;
            }

            if !matches_core_major
                && !matches!(&frame, Frame::Control(control) if control.is_frozen())
            {
                continue;
            }

            self.route(inbound.connection, frame, inbound.batch, &mut active);
        }
    }

    /// Routes one frame. `active` records connection activity reported by earlier frames in the batch.
    pub(super) fn route(
        self: &Rc<Self>,
        connection: Option<ConnectionId>,
        frame: Frame,
        batch: &Rc<[u8]>,
        active: &mut bool,
    ) {
        if let Frame::Control(control) = frame {
            *active = false;
            self.on_control(*control, connection);

            return;
        }

        let Some((connection, session)) = connection.and_then(|connection| {
            self.session_of(connection)
                .map(|session| (connection, session))
        }) else {
            return;
        };

        if !std::mem::replace(active, true) {
            self.on_activity();
        }

        let body = |range| Body::new(Rc::clone(batch), range, session.minor());

        match frame {
            Frame::Message {
                type_id,
                id,
                body: range,
            } => self.on_message(connection, &session, type_id, id, body(range)),
            Frame::Segment {
                segment,
                id,
                body: range,
            } if segment.is_response() => self.on_response(&session, segment, id, body(range)),
            Frame::Segment {
                segment,
                id,
                body: range,
            } => self.on_message_stream(&session, segment, id, body(range)),
            Frame::Rejection { id, rejection } => {
                let exchange_id = self.messages.borrow().outbound_id(id);

                if let Some(id) = exchange_id {
                    let error = rejection.error();

                    self.release_on_settle(id, Event::Error(error.clone()), &error);
                }
            }
            Frame::Credit {
                for_message,
                id,
                count,
            } => {
                let mut messages = self.messages.borrow_mut();
                let target = if for_message {
                    messages.outbound_id(id)
                } else {
                    messages.inbound_id(id)
                };

                if let Some(output) = target
                    .and_then(|id| messages.get_mut(id))
                    .and_then(Message::output)
                {
                    output.grant(count);
                }
            }
            Frame::Cancel { id } => {
                let target = self.messages.borrow().inbound_id(id);

                if let Some(id) = target {
                    self.reject_inbound(id, ErrorCode::Cancellation);
                }
            }
            Frame::Control(_) | Frame::Fragment { .. } => {}
        }
    }

    fn on_message(
        self: &Rc<Self>,
        connection: ConnectionId,
        session: &Session,
        remote_type_id: u32,
        exchange_id: Option<u32>,
        body: Body,
    ) {
        let (type_id, body) = match session.read_message(remote_type_id, body) {
            Ok(read) => read,
            Err(code) => {
                if let Some(exchange_id) = exchange_id {
                    self.write_rejection(connection, exchange_id, code);
                }

                return;
            }
        };
        let Some(exchange_id) = exchange_id else {
            if !self.handlers.handles_notification(type_id) {
                return;
            }

            let requirements = Self::inbound_requirements(type_id);
            let id = self.messages.borrow_mut().push_notification(Notification {
                type_id,
                connection,
                limit: requirements.limit,
                body,
            });

            self.enqueue(Work::Notification(id));

            return;
        };
        let route = self.handlers.message(type_id);
        let streams = route.and_then(|route| {
            let input = match route.streaming {
                Streaming::Items { max_size } => {
                    Some(ReceiveWindow::new(self.settings.window, max_size))
                }
                Streaming::No => None,
            };
            let output = route.has_response.then_some(INITIAL_CREDIT);

            StreamPair::of(remote_type_id, input, output)
        });
        let deadline = self
            .timeout_of(type_id)
            .and_then(|timeout| Some(self.now()? + timeout));
        let ready = InboundState::Ready {
            connection,
            body,
            deadline,
        };
        let inbound = Message::new(type_id, Flow::Inbound(ready), streams);
        let arrival = self.messages.borrow_mut().receive(
            exchange_id,
            self.settings.limits.incoming_items,
            inbound,
        );
        let id = match arrival {
            Arrival::New(id) => id,
            Arrival::Full => {
                return self.write_rejection(connection, exchange_id, ErrorCode::Overload);
            }
            Arrival::Duplicate => return,
        };

        if let Some(deadline) = deadline {
            self.schedule_deadline(deadline);
        }

        self.enqueue(Work::Message(id));
    }

    fn enqueue(&self, work: Work) {
        if let Some(transport) = self.transport.upgrade() {
            transport.enqueue(self.endpoint(), work);
        }
    }

    pub(crate) fn handle_notification(&self, id: NotificationId) {
        let Some(mut notification) = self.messages.borrow_mut().take_notification(id) else {
            return;
        };

        if !self.is_current(notification.connection) {
            return;
        }

        if let Some(dispatch) = self.handlers.notification(notification.type_id)
            && (!Self::VALIDATES_INPUT
                || notification
                    .body
                    .validate(&Validation::Any, notification.limit)
                    .is_ok())
        {
            dispatch(&notification.body);
        }
    }

    pub(crate) fn handle_message(&self, id: EntryId) {
        let start = {
            let mut messages = self.messages.borrow_mut();

            if messages.active_response_count() >= self.settings.limits.active_responses {
                Err(ErrorCode::Overload)
            } else {
                let Some((message, body)) = messages.process(id) else {
                    return;
                };

                match self.handlers.message_dispatch(message) {
                    Some((dispatch, refs)) => {
                        let reads = messages.add_refs(id, refs);

                        Ok((message, body, dispatch, refs, reads))
                    }
                    None => Err(ErrorCode::UnhandledMessage),
                }
            }
        };
        let (message, mut body, dispatch, refs, reads) = match start {
            Ok(start) => start,
            Err(code) => return self.reject_inbound(id, code),
        };
        let limit = Self::inbound_requirements(message).limit;

        if Self::VALIDATES_INPUT
            && let Err(error) = body.validate(&Validation::Any, limit)
        {
            return self.handle_message_rejection(id, Rejection::of(&error), refs);
        }

        dispatch(self, id, &body, reads);
    }

    pub(crate) fn handle_message_rejection(&self, id: EntryId, rejection: Rejection, refs: u8) {
        self.reject_inbound(id, rejection);
        self.messages.borrow_mut().release_by(id, refs);
    }

    fn on_response(
        &self,
        session: &Session,
        segment: MessageSegment,
        exchange_id: u32,
        body: Body,
    ) {
        if !Self::VALIDATES_INPUT && session.migration_path.is_none() && segment.is_response_end() {
            let event = if segment == MessageSegment::Failure {
                Event::Failure(body)
            } else {
                Event::End(body)
            };

            self.messages.borrow_mut().handle_event_via_frame(
                exchange_id,
                event,
                &Error::ChannelEnd(ErrorCode::ClosedChannel),
            );

            return;
        }

        let Some(FrameTarget {
            id,
            type_id,
            requirements,
            item_count,
        }) = self.outbound_target(exchange_id)
        else {
            return;
        };
        let payload = session
            .read_segment(segment, type_id, body)
            .and_then(|mut body| {
                if Self::VALIDATES_INPUT {
                    body.validate(requirements.of(segment), requirements.limit)?;
                    Self::validate_count(requirements, segment, item_count)?;
                }

                Ok(body)
            });

        if segment.is_response_end() {
            let event = match payload {
                Ok(body) if segment == MessageSegment::Failure => Event::Failure(body),
                Ok(body) => Event::End(body),
                Err(error) => Event::Error(error),
            };

            self.release_on_settle(id, event, &Error::ChannelEnd(ErrorCode::ClosedChannel));

            return;
        }

        let failed = {
            let mut messages = self.messages.borrow_mut();
            let Some(entry) = messages.get_mut(id) else {
                return;
            };

            if !matches!(
                entry.flow,
                Flow::Outbound(OutboundState::ResponsePending(_))
            ) {
                return;
            }

            let Some(input) = entry.input() else {
                return;
            };

            match payload {
                Ok(body) if segment == MessageSegment::ResponseStart => {
                    input.push(Event::Start(body));

                    false
                }
                Ok(body) => {
                    input.push(Event::Item(body));
                    if Self::VALIDATES_INPUT {
                        entry.count_item(RESPONSE_STREAM);
                    }

                    false
                }
                Err(error) => {
                    input.push(Event::Error(error));

                    true
                }
            }
        };

        if failed {
            self.cancel_outbound(id, ErrorCode::Cancellation);
        }
    }

    fn on_message_stream(
        &self,
        session: &Session,
        segment: MessageSegment,
        exchange_id: u32,
        body: Body,
    ) {
        let Some(FrameTarget {
            id,
            type_id,
            requirements,
            item_count,
        }) = self.input_target(exchange_id)
        else {
            return;
        };
        let read = session
            .read_segment(segment, type_id, body)
            .and_then(|mut body| {
                if Self::VALIDATES_INPUT {
                    body.validate(requirements.of(segment), requirements.limit)?;
                    Self::validate_count(requirements, segment, item_count)?;
                }

                Ok(body)
            });
        let rejection = {
            let mut messages = self.messages.borrow_mut();
            let Some(message) = messages.get_mut(id) else {
                return;
            };

            if Self::VALIDATES_INPUT && read.is_ok() && segment == MessageSegment::MessageItem {
                message.count_item(MESSAGE_STREAM);
            }

            let Some(input) = message.input() else {
                return;
            };

            match read {
                Ok(body) if segment == MessageSegment::MessageEnd => {
                    input.push(Event::End(body));

                    None
                }
                Ok(body) => {
                    input.push(Event::Item(body));

                    None
                }
                Err(error) => {
                    let rejection = Rejection::of(&error);

                    input.push(Event::Error(error));

                    Some(rejection)
                }
            }
        };

        if let Some(rejection) = rejection {
            self.reject_inbound(id, rejection);
        }
    }

    fn outbound_target(&self, exchange_id: u32) -> Option<FrameTarget> {
        let messages = self.messages.borrow();
        let id = messages.outbound_id(exchange_id)?;
        let message = messages.get(id)?;

        Some(FrameTarget {
            id,
            type_id: message.type_id,
            requirements: Self::outbound_requirements(message.type_id),
            item_count: message.items(RESPONSE_STREAM),
        })
    }

    /// Resolves inbound message stream `exchange_id` and refreshes its deadline.
    fn input_target(&self, exchange_id: u32) -> Option<FrameTarget> {
        let mut messages = self.messages.borrow_mut();
        let id = messages.inbound_id(exchange_id)?;
        let message = messages.get_mut(id)?;
        let streams = message
            .streams
            .as_ref()
            .filter(|streams| streams.input.is_some())?;
        let target = FrameTarget {
            id,
            type_id: streams.remote_type_id,
            requirements: Self::inbound_requirements(message.type_id),
            item_count: streams.items[MESSAGE_STREAM],
        };

        self.refresh_deadline_of(message);

        Some(target)
    }

    /// Validates item or end `segment` against declared stream item-count requirements.
    fn validate_count(
        requirements: &MessageRequirementTable,
        segment: MessageSegment,
        item_count: u32,
    ) -> Result<(), Error> {
        match segment {
            MessageSegment::MessageItem | MessageSegment::ResponseItem => requirements
                .items_of(segment)
                .is_less_than(item_count as usize + 1),
            MessageSegment::MessageEnd => requirements
                .items_of(MessageSegment::MessageItem)
                .is_within(item_count as usize),
            MessageSegment::ResponseEnd => requirements
                .items_of(MessageSegment::ResponseItem)
                .is_within(item_count as usize),
            _ => Ok(()),
        }
    }
}
