use std::fmt::Debug;
use std::future::{Future, poll_fn};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

use super::TypeMarker;
use super::io::{IncomingExchange, MessageRef, OutgoingExchange, Release};
use crate::binary::ValidatedDecode;
use crate::channel::message::{Body, Event};
use crate::definition::ErrorCode;
use crate::error::{DecodeFailure, Error};
use crate::frame::Rejection;

enum FinalEvent {
    End(Body),
    Failure(Body),
    Error(Error),
}

impl FinalEvent {
    fn of(event: Event) -> Option<Self> {
        match event {
            Event::End(body) => Some(Self::End(body)),
            Event::Failure(body) => Some(Self::Failure(body)),
            Event::Error(error) => Some(Self::Error(error)),
            Event::Start(_) | Event::Item(_) => None,
        }
    }

    fn read<V: ValidatedDecode, F: ValidatedDecode + Debug>(&self) -> Result<V, Error<F>> {
        match (self, self.failure()) {
            (_, Some(failure)) => Err(failure),
            (Self::End(body), None) => body.decode().map_err(Error::with_failure),
            _ => unreachable!("only an end is no failure"),
        }
    }

    fn failure<F: ValidatedDecode + Debug>(&self) -> Option<Error<F>> {
        match self {
            Self::End(_) => None,
            Self::Failure(body) => Some(match body.decode() {
                Ok(failure) => Error::Failure(failure),
                Err(error) => error.with_failure(),
            }),
            Self::Error(error) => Some(error.clone().with_failure()),
        }
    }
}

struct Reader(MessageRef);

impl Reader {
    fn new(channel: std::rc::Weak<dyn super::io::ChannelIo>, id: crate::keys::EntryId) -> Self {
        Self(MessageRef::new(channel, id, Release::Input))
    }

    fn poll_next(&self, context: &mut Context<'_>) -> Poll<Event> {
        self.0
            .poll(|channel, id| channel.poll_event(id, context), Event::Error)
    }

    fn cancel(&self) {
        self.0.with(|channel, id| channel.cancel_message(id));
    }

    fn decode<T: ValidatedDecode>(&self, body: &Body) -> Result<T, Error> {
        let value = body.decode();

        if let Err(error) = &value {
            self.0
                .with(|channel, id| channel.fail_input(id, Rejection::of(error)));
        }

        value
    }
}

/// Future resolving to a message response value or declared failure.
/// Drop before completion cancels the message.
pub struct PendingResponse<R, F: Debug = std::convert::Infallible> {
    message: MessageRef,
    settled: bool,
    value: TypeMarker<(R, F)>,
}

impl<R, F: Debug> PendingResponse<R, F> {
    pub(crate) fn new(exchange: OutgoingExchange) -> Self {
        Self {
            message: MessageRef::new(exchange.channel, exchange.message, Release::Message),
            settled: false,
            value: PhantomData,
        }
    }
}

impl<R: ValidatedDecode, F: ValidatedDecode + Debug> Future for PendingResponse<R, F> {
    type Output = Result<R, Error<F>>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let Poll::Ready(event) = this.message.poll(
            |channel, id| channel.poll_outcome(id, context),
            Event::Error,
        ) else {
            return Poll::Pending;
        };

        this.settled = true;
        this.message.release();

        Poll::Ready(match FinalEvent::of(event) {
            Some(final_event) => final_event.read(),
            None => Err(Error::Decode(DecodeFailure::ResponseEnd)),
        })
    }
}

impl<R, F: Debug> Drop for PendingResponse<R, F> {
    fn drop(&mut self) {
        if !self.settled {
            self.message.with(|channel, id| channel.cancel_message(id));
        }
    }
}

/// Streamed response with a start value, items, and an end value or declared failure.
/// Drop before completion cancels the message.
pub struct ResponseStream<S, E, I, F: Debug = std::convert::Infallible> {
    reader: Reader,
    final_event: Option<FinalEvent>,
    values: TypeMarker<(S, E, I, F)>,
}

impl<S, E, I, F> ResponseStream<S, E, I, F>
where
    S: ValidatedDecode,
    E: ValidatedDecode,
    I: ValidatedDecode,
    F: ValidatedDecode + Debug,
{
    pub(crate) fn new(exchange: OutgoingExchange) -> Self {
        Self {
            reader: Reader::new(exchange.channel, exchange.message),
            final_event: None,
            values: PhantomData,
        }
    }

    /// Waits for and decodes the response start value.
    ///
    /// # Errors
    ///
    /// Returns `Failure` for the peer's declared failure, `Rejection` if the peer rejects the message, `Decode` or
    /// `Invalid` for an unreadable response, and `ChannelEnd` if the channel ends first.
    pub async fn start(&mut self) -> Result<S, Error<F>> {
        loop {
            if let Some(final_event) = &self.final_event {
                return Err(final_event
                    .failure()
                    .unwrap_or_else(|| Error::Decode(DecodeFailure::ResponseStart)));
            }

            match poll_fn(|context| self.reader.poll_next(context)).await {
                Event::Start(body) => return body.decode().map_err(Error::with_failure),
                Event::Item(_) => {
                    return Err(Error::Decode(DecodeFailure::ResponseItemOrder));
                }
                event => self.final_event = FinalEvent::of(event),
            }
        }
    }

    /// Returns the next response item, or `None` when the response has ended.
    pub async fn next(&mut self) -> Option<Result<I, Error<F>>> {
        poll_fn(|context| self.poll_item(context)).await
    }

    /// Drains remaining items and returns the response end value.
    ///
    /// # Errors
    ///
    /// Returns `Failure` for the peer's declared failure, `Rejection` if the peer rejects the message, `Decode` or
    /// `Invalid` for an unreadable response, and `ChannelEnd` if the channel ends first.
    pub async fn end(mut self) -> Result<E, Error<F>> {
        while self.next().await.is_some() {}

        self.final_event.as_ref().map_or(
            Err(Error::ChannelEnd(ErrorCode::ClosedChannel)),
            FinalEvent::read,
        )
    }

    fn poll_item(&mut self, context: &mut Context<'_>) -> Poll<Option<Result<I, Error<F>>>> {
        if self.final_event.is_some() {
            return Poll::Ready(None);
        }

        loop {
            let Poll::Ready(event) = self.reader.poll_next(context) else {
                return Poll::Pending;
            };

            match event {
                Event::Start(_) => {}
                Event::Item(body) => {
                    return Poll::Ready(Some(
                        self.reader.decode(&body).map_err(Error::with_failure),
                    ));
                }
                event => {
                    let Some(final_event) = FinalEvent::of(event) else {
                        continue;
                    };
                    let failure = final_event.failure();

                    self.final_event = Some(final_event);

                    return Poll::Ready(failure.map(Err));
                }
            }
        }
    }
}

impl<S, E, I, F> Stream for ResponseStream<S, E, I, F>
where
    S: ValidatedDecode,
    E: ValidatedDecode,
    I: ValidatedDecode,
    F: ValidatedDecode + Debug,
{
    type Item = Result<I, Error<F>>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().poll_item(context)
    }
}

impl<S, E, I, F: Debug> Drop for ResponseStream<S, E, I, F> {
    fn drop(&mut self) {
        if !matches!(
            self.final_event,
            Some(FinalEvent::End(_) | FinalEvent::Failure(_))
        ) {
            self.reader.cancel();
        }
    }
}

/// Incoming stream that follows the remote peer's message.
pub struct IncomingStream<E, I> {
    reader: Reader,
    result: Option<Result<Body, Error>>,
    values: TypeMarker<(E, I)>,
}

impl<E: ValidatedDecode, I: ValidatedDecode> IncomingStream<E, I> {
    pub(crate) fn new(exchange: &IncomingExchange) -> Self {
        Self {
            reader: Reader::new(std::rc::Weak::clone(&exchange.channel), exchange.message),
            result: None,
            values: PhantomData,
        }
    }

    /// Returns the next stream item, or `None` when the stream has ended.
    pub async fn next(&mut self) -> Option<Result<I, Error>> {
        poll_fn(|context| self.poll_item(context)).await
    }

    /// Drains remaining items and returns the stream end value.
    ///
    /// # Errors
    ///
    /// Returns `Decode` or `Invalid` for an unreadable stream and `ChannelEnd` if the channel ends first.
    pub async fn end(mut self) -> Result<E, Error> {
        while self.next().await.is_some() {}

        match self.result.take() {
            Some(Ok(body)) => body.decode(),
            Some(Err(error)) => Err(error),
            None => Err(Error::ChannelEnd(ErrorCode::ClosedChannel)),
        }
    }

    fn poll_item(&mut self, context: &mut Context<'_>) -> Poll<Option<Result<I, Error>>> {
        if self.result.is_some() {
            return Poll::Ready(None);
        }

        loop {
            let Poll::Ready(event) = self.reader.poll_next(context) else {
                return Poll::Pending;
            };

            match event {
                Event::Item(body) => {
                    return Poll::Ready(Some(self.reader.decode(&body)));
                }
                Event::End(body) => {
                    self.result = Some(Ok(body));

                    return Poll::Ready(None);
                }
                Event::Error(error) => {
                    self.result = Some(Err(error.clone()));

                    return Poll::Ready(Some(Err(error)));
                }
                Event::Start(_) | Event::Failure(_) => {}
            }
        }
    }
}

impl<E: ValidatedDecode, I: ValidatedDecode> Stream for IncomingStream<E, I> {
    type Item = Result<I, Error>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().poll_item(context)
    }
}
