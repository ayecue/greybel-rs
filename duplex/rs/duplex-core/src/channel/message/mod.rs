mod body;
mod ids;
mod stream;
mod table;
mod window;

use std::collections::VecDeque;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

pub(crate) use body::Body;
pub(crate) use stream::{Event, InputHandle, OutputHandle};
pub(crate) use table::{Arrival, MessageTable};
pub(crate) use window::{INITIAL_CREDIT, ReceiveWindow};

use crate::binary::{BinaryReader, BinaryWriter};
use crate::channel::Bundle;
use crate::connection_id::ConnectionId;
use crate::definition::ErrorCode;
use crate::error::Error;

type Transcode = fn(&mut BinaryReader<'_>, &mut BinaryWriter) -> Result<(), Error>;

/// Send-time encoded message body queued until a connection is available.
pub(crate) struct PendingBody {
    pub(crate) body: Vec<u8>,
    pub(crate) since: u16,
    pub(crate) transcode: Transcode,
    pub(crate) bundle: Bundle,
}

impl PendingBody {
    pub(crate) fn write_body(&self, writer: &mut BinaryWriter, newest: u16) -> Result<(), Error> {
        if writer.minor() >= newest {
            writer.raw(&self.body);

            return Ok(());
        }

        (self.transcode)(&mut BinaryReader::new(&self.body), writer)
    }
}

/// Received message state from initial frame through response completion.
pub(crate) enum InboundState {
    Ready {
        connection: ConnectionId,
        body: Body,
        deadline: Option<Duration>,
    },
    HandlerActive {
        connection: ConnectionId,
        deadline: Option<Duration>,
    },
    Complete,
}

/// A final response recorded on arrival and waiting behind earlier transport work.
pub(crate) struct PendingSettlement {
    event: Event,
    stream_error: Error,
    waker: Option<Waker>,
    cancellation: bool,
    outcome: Option<Event>,
}

/// Locally sent message state from `send` through response consumption.
/// Dispatch- and response-pending states store the sender waker.
pub(crate) enum OutboundState {
    Pending(PendingBody, Option<Waker>),
    Unsent(Option<Waker>),
    ResponsePending(Option<Waker>),
    DeliveryPending(PendingSettlement),
    /// Cancellation pending. Retains the exchange ID until the remote response and buffers any unconsumed outcome.
    CancelPending(Option<Event>),
    /// Final outcome waiting for sender consumption.
    Outcome(Option<Event>),
}

impl OutboundState {
    fn into_waker(self) -> Option<Waker> {
        match self {
            Self::Pending(_, waker) | Self::Unsent(waker) | Self::ResponsePending(waker) => waker,
            Self::DeliveryPending(settlement) => settlement.waker,
            Self::CancelPending(_) | Self::Outcome(_) => None,
        }
    }

    /// Changes `Unsent` to `ResponsePending` after writing the first frame.
    /// Returns whether the state changed.
    pub(crate) fn finish_dispatch(&mut self) -> bool {
        let Self::Unsent(waker) = self else {
            return false;
        };

        *self = Self::ResponsePending(waker.take());

        true
    }

    /// Changes `Pending` to `Unsent` and returns the queued body.
    pub(crate) fn dequeue(&mut self) -> Option<PendingBody> {
        let Self::Pending(_, waker) = self else {
            return None;
        };
        let waker = waker.take();

        match std::mem::replace(self, Self::Unsent(waker)) {
            Self::Pending(pending_body, _) => Some(pending_body),
            _ => None,
        }
    }
}

pub(crate) enum Flow {
    Inbound(InboundState),
    Outbound(OutboundState),
    /// Notification queued until a connection becomes available.
    Notification(PendingBody),
}

/// Message stream index in `StreamPair::items`.
pub(crate) const MESSAGE_STREAM: usize = 0;
/// Response stream index in `StreamPair::items`.
pub(crate) const RESPONSE_STREAM: usize = 1;

/// Message input/output stream state and item counts.
pub(crate) struct StreamPair {
    pub(crate) remote_type_id: u32,
    pub(crate) items: [u32; 2],
    pub(crate) input: Option<InputHandle>,
    pub(crate) output: Option<OutputHandle>,
}

impl StreamPair {
    pub(crate) fn of(
        remote_type_id: u32,
        input: Option<ReceiveWindow>,
        output: Option<u32>,
    ) -> Option<Box<Self>> {
        if input.is_none() && output.is_none() {
            return None;
        }

        Some(Box::new(Self {
            remote_type_id,
            items: [0; 2],
            input: input.map(InputHandle::new),
            output: output.map(OutputHandle::new),
        }))
    }
}

/// Received notification waiting for handler dispatch.
pub(crate) struct Notification {
    pub(crate) type_id: u32,
    pub(crate) connection: ConnectionId,
    pub(crate) limit: u32,
    pub(crate) body: Body,
}

/// Message exchange state with flow data and optional stream state.
pub(crate) struct Message {
    pub(crate) exchange_id: u32,
    pub(crate) type_id: u32,
    pub(crate) refs: u8,
    pub(crate) flow: Flow,
    pub(crate) streams: Option<Box<StreamPair>>,
    pending_events: VecDeque<Event>,
}

impl Message {
    pub(crate) fn new(type_id: u32, flow: Flow, streams: Option<Box<StreamPair>>) -> Self {
        Self {
            exchange_id: 0,
            type_id,
            refs: 0,
            flow,
            streams,
            pending_events: VecDeque::new(),
        }
    }

    pub(crate) fn is_outbound(&self) -> bool {
        matches!(self.flow, Flow::Outbound(_))
    }

    /// Returns the connection ID for a ready or handler-active inbound message.
    pub(crate) fn connection(&self) -> Option<ConnectionId> {
        match self.flow {
            Flow::Inbound(
                InboundState::Ready { connection, .. }
                | InboundState::HandlerActive { connection, .. },
            ) => Some(connection),
            _ => None,
        }
    }

    pub(crate) fn is_ready(&self) -> bool {
        matches!(self.flow, Flow::Inbound(InboundState::Ready { .. }))
    }

    pub(crate) fn is_active(&self) -> bool {
        self.connection().is_some()
    }

    pub(crate) fn deadline(&self) -> Option<Duration> {
        match self.flow {
            Flow::Inbound(
                InboundState::Ready { deadline, .. } | InboundState::HandlerActive { deadline, .. },
            ) => deadline,
            _ => None,
        }
    }

    pub(crate) fn set_deadline(&mut self, at: Duration) {
        if let Flow::Inbound(
            InboundState::Ready { deadline, .. } | InboundState::HandlerActive { deadline, .. },
        ) = &mut self.flow
        {
            *deadline = Some(at);
        }
    }

    pub(crate) fn input(&mut self) -> Option<&mut InputHandle> {
        self.streams.as_mut()?.input.as_mut()
    }

    pub(crate) fn output(&mut self) -> Option<&mut OutputHandle> {
        self.streams.as_mut()?.output.as_mut()
    }

    pub(crate) fn items(&self, stream: usize) -> u32 {
        self.streams
            .as_ref()
            .map_or(0, |streams| streams.items[stream])
    }

    /// Increments and returns the item count for `stream`, or returns 0 if stream state is absent.
    pub(crate) fn count_item(&mut self, stream: usize) -> u32 {
        self.streams.as_mut().map_or(0, |streams| {
            streams.items[stream] += 1;

            streams.items[stream]
        })
    }

    pub(crate) fn fail_stream(&mut self, error: &Error) {
        if let Some(input) = self.input() {
            input.push(Event::Error(error.clone()));
        }

        if let Some(output) = self.output() {
            output.fail(error.clone());
        }
    }

    pub(crate) fn fail(&mut self, error: &Error) {
        self.fail_stream(error);
        self.flow = Flow::Inbound(InboundState::Complete);
    }

    /// Records a response stream event for delivery by its queued work item.
    pub(crate) fn defer_response_item(&mut self, event: Event) -> Result<(), Event> {
        if !matches!(self.flow, Flow::Outbound(OutboundState::ResponsePending(_))) {
            return Err(event);
        }

        self.pending_events.push_back(event);

        Ok(())
    }

    /// Delivers the oldest response stream event, returning whether it was an error.
    pub(crate) fn deliver_response_item(&mut self) -> Option<bool> {
        let event = self.pending_events.pop_front()?;
        let failed = matches!(event, Event::Error(_));

        if matches!(
            self.flow,
            Flow::Outbound(
                OutboundState::ResponsePending(_)
                    | OutboundState::DeliveryPending(_)
                    | OutboundState::CancelPending(_)
            )
        ) && let Some(input) = self.input()
        {
            input.push(event);
        }

        Some(failed)
    }

    /// Records a final response without waking its sender before earlier work is handled.
    pub(crate) fn receive_response(&mut self, event: Event, stream_error: &Error) -> bool {
        let Flow::Outbound(outbound) = &mut self.flow else {
            return false;
        };
        let (waker, cancellation, outcome) =
            match std::mem::replace(outbound, OutboundState::Outcome(None)) {
                OutboundState::ResponsePending(waker) => (waker, false, None),
                OutboundState::CancelPending(outcome) => (None, true, outcome),
                other => {
                    *outbound = other;

                    return false;
                }
            };

        *outbound = OutboundState::DeliveryPending(PendingSettlement {
            event,
            stream_error: stream_error.clone(),
            waker,
            cancellation,
            outcome,
        });

        true
    }

    /// Delivers a final response previously recorded by [`Message::receive_response`].
    pub(crate) fn deliver_response(&mut self) {
        let Flow::Outbound(OutboundState::DeliveryPending(_)) = &self.flow else {
            return;
        };
        let Flow::Outbound(OutboundState::DeliveryPending(settlement)) =
            std::mem::replace(&mut self.flow, Flow::Outbound(OutboundState::Outcome(None)))
        else {
            unreachable!()
        };

        if let Some(output) = self.output() {
            output.fail(settlement.stream_error);
        }

        if settlement.cancellation {
            self.emit(settlement.outcome, None);
        } else {
            self.emit(Some(settlement.event), settlement.waker);
        }
    }

    pub(crate) fn on_event(&mut self, event: Event, stream_error: &Error) {
        if let Some(output) = self.output() {
            output.fail(stream_error.clone());
        }

        match std::mem::replace(&mut self.flow, Flow::Outbound(OutboundState::Outcome(None))) {
            Flow::Outbound(OutboundState::CancelPending(outcome)) => self.emit(outcome, None),
            Flow::Outbound(outbound_state) => self.emit(Some(event), outbound_state.into_waker()),
            _ => {}
        }
    }

    pub(crate) fn fail_stream_and_emit(&mut self, error: &Error, waits: bool) {
        if let Some(output) = self.output() {
            output.fail(error.clone());
        }

        let waker =
            match std::mem::replace(&mut self.flow, Flow::Outbound(OutboundState::Outcome(None))) {
                Flow::Outbound(outbound_state) => outbound_state.into_waker(),
                _ => None,
            };
        let outcome = Some(Event::Error(error.clone()));

        if !waits {
            return self.emit(outcome, waker);
        }

        let outcome = match self.input() {
            Some(input) => {
                input.push(Event::Error(error.clone()));

                None
            }
            None => outcome,
        };

        self.flow = Flow::Outbound(OutboundState::CancelPending(outcome));
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn emit(&mut self, outcome: Option<Event>, waker: Option<Waker>) {
        match (self.input(), outcome) {
            (Some(input), Some(event)) => input.push(event),
            (None, outcome) => match &mut self.flow {
                Flow::Outbound(OutboundState::Outcome(outcome_slot)) => *outcome_slot = outcome,
                flow => *flow = Flow::Outbound(OutboundState::Outcome(outcome)),
            },
            (Some(_), None) => {}
        }

        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// Polls and consumes a ready response outcome, registering the sender waker while pending.
    pub(crate) fn poll_outcome(&mut self, context: &mut Context<'_>) -> Poll<Event> {
        let closed_event = || Event::Error(Error::ChannelEnd(ErrorCode::ClosedChannel));
        let Flow::Outbound(outbound) = &mut self.flow else {
            return Poll::Ready(closed_event());
        };

        match outbound {
            OutboundState::Outcome(outcome) | OutboundState::CancelPending(outcome) => {
                Poll::Ready(outcome.take().unwrap_or_else(closed_event))
            }
            OutboundState::Pending(_, waker)
            | OutboundState::Unsent(waker)
            | OutboundState::ResponsePending(waker) => {
                *waker = Some(context.waker().clone());

                Poll::Pending
            }
            OutboundState::DeliveryPending(settlement) if settlement.cancellation => {
                Poll::Ready(settlement.outcome.take().unwrap_or_else(closed_event))
            }
            OutboundState::DeliveryPending(settlement) => {
                settlement.waker = Some(context.waker().clone());

                Poll::Pending
            }
        }
    }

    fn is_removable(&self) -> bool {
        self.refs == 0
            && matches!(
                self.flow,
                Flow::Inbound(InboundState::Complete) | Flow::Outbound(OutboundState::Outcome(_))
            )
    }
}
