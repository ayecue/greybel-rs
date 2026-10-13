use std::borrow::Borrow;
use std::future::poll_fn;
use std::iter::Peekable;
use std::marker::PhantomData;

use super::TypeMarker;
use super::io::{IncomingExchange, ItemSource, MessageRef, OutgoingExchange, Release};
use crate::binary::ValidatedEncode;
use crate::definition::ErrorCode;
use crate::error::Error;
use crate::protocol::MessageSegment;

struct Respond {
    message: MessageRef,
    settled: bool,
}

impl Respond {
    fn new(exchange: &IncomingExchange) -> Self {
        Self {
            message: MessageRef::new(
                std::rc::Weak::clone(&exchange.channel),
                exchange.message,
                Release::Message,
            ),
            settled: false,
        }
    }

    fn is_finished(&self) -> bool {
        self.settled
            || self
                .message
                .query(|channel, id| channel.is_finished(id))
                .unwrap_or(true)
    }

    fn segment(&self, segment: MessageSegment, value: &dyn ValidatedEncode) -> Result<(), Error> {
        if self.settled {
            return Err(Error::ChannelEnd(ErrorCode::ClosedChannel));
        }

        self.message
            .try_with(|channel, id| channel.respond(id, segment, value))
    }

    async fn item(&self, value: &dyn ValidatedEncode) -> Result<(), Error> {
        if self.settled {
            return Err(Error::ChannelEnd(ErrorCode::ClosedChannel));
        }

        poll_fn(|context| {
            self.message.poll(
                |channel, id| channel.poll_response_item(id, value, context),
                Err,
            )
        })
        .await
    }

    fn settle(&mut self, segment: MessageSegment, value: &dyn ValidatedEncode) {
        if !std::mem::replace(&mut self.settled, true) {
            self.message
                .with(|channel, id| channel.settle(id, segment, value));
            self.message.release();
        }
    }

    fn reject(&mut self, code: ErrorCode) {
        if !std::mem::replace(&mut self.settled, true) {
            self.message
                .with(|channel, id| channel.reject(id, code.into()));
            self.message.release();
        }
    }
}

impl Drop for Respond {
    fn drop(&mut self) {
        self.reject(ErrorCode::DroppedMessage);
    }
}

struct Writer(MessageRef);

impl Writer {
    fn new(channel: std::rc::Weak<dyn super::io::ChannelIo>, id: crate::keys::EntryId) -> Self {
        Self(MessageRef::new(channel, id, Release::Output))
    }

    async fn item(&self, value: &dyn ValidatedEncode) -> Result<(), Error> {
        poll_fn(|context| {
            self.0.poll(
                |channel, id| channel.poll_stream_item(id, value, context),
                Err,
            )
        })
        .await
    }

    async fn open(&self) -> Result<(), Error> {
        poll_fn(|context| {
            self.0
                .poll(|channel, id| channel.poll_open(id, context), Err)
        })
        .await
    }

    fn write(&self, segment: MessageSegment, value: &dyn ValidatedEncode) -> Result<(), Error> {
        self.0
            .try_with(|channel, id| channel.write_stream(id, segment, value))
    }

    fn cancel(&self) {
        self.0.with(|channel, id| channel.cancel_message(id));
    }

    async fn write_all<I: ValidatedEncode, T: Borrow<I>>(
        &self,
        items: impl IntoIterator<Item = T>,
    ) -> Result<(), Error> {
        let mut batch = ItemIterator::<I, _> {
            items: items.into_iter().peekable(),
            values: PhantomData,
        };

        while batch.items.peek().is_some() {
            poll_fn(|context| {
                self.0
                    .poll(|channel, id| channel.poll_available(id, context), Err)
            })
            .await?;
            self.0
                .try_with(|channel, id| channel.write_items(id, &mut batch))?;
        }

        Ok(())
    }
}

struct ItemIterator<I, J: Iterator> {
    items: Peekable<J>,
    values: TypeMarker<I>,
}

impl<I: ValidatedEncode, T: Borrow<I>, J: Iterator<Item = T>> ItemSource for ItemIterator<I, J> {
    fn write_next(
        &mut self,
        write: &mut dyn FnMut(&dyn ValidatedEncode) -> Result<(), Error>,
    ) -> Option<Result<(), Error>> {
        let item = self.items.next()?;

        Some(write(item.borrow()))
    }
}

/// Sends one response to a received message.
/// Drop without a response rejects the message as `DroppedMessage`.
pub struct Responder<R, F = std::convert::Infallible> {
    respond: Respond,
    values: TypeMarker<(R, F)>,
}

impl<R: ValidatedEncode, F: ValidatedEncode> Responder<R, F> {
    pub(crate) fn new(exchange: &IncomingExchange) -> Self {
        Self {
            respond: Respond::new(exchange),
            values: PhantomData,
        }
    }

    #[must_use]
    /// Returns whether the sender canceled the message.
    pub fn is_finished(&self) -> bool {
        self.respond.is_finished()
    }

    /// Sends a successful response value.
    pub fn respond(mut self, response: R) {
        self.respond.settle(MessageSegment::ResponseEnd, &response);
    }

    /// Sends a declared failure value.
    pub fn fail(mut self, failure: F) {
        self.respond.settle(MessageSegment::Failure, &failure);
    }

    pub(crate) fn reject(mut self, code: ErrorCode) {
        self.respond.reject(code);
    }
}

/// Sends a streamed response or declared failure for a received message.
pub struct StreamResponder<S, E, I, F = std::convert::Infallible> {
    respond: Respond,
    response: Writer,
    values: TypeMarker<(S, E, I, F)>,
}

impl<S: ValidatedEncode, E: ValidatedEncode, I: ValidatedEncode, F: ValidatedEncode>
    StreamResponder<S, E, I, F>
{
    pub(crate) fn new(exchange: &IncomingExchange) -> Self {
        Self {
            respond: Respond::new(exchange),
            response: Writer::new(std::rc::Weak::clone(&exchange.channel), exchange.message),
            values: PhantomData,
        }
    }

    #[must_use]
    /// Returns whether the sender canceled the message.
    pub fn is_finished(&self) -> bool {
        self.respond.is_finished()
    }

    /// Starts the response stream and returns the item writer.
    pub fn start(self, start: S) -> ItemWriter<E, I, F> {
        let Self {
            respond, response, ..
        } = self;
        let failure = respond.segment(MessageSegment::ResponseStart, &start).err();

        ItemWriter {
            respond,
            response,
            failure,
            values: PhantomData,
        }
    }

    /// Sends a declared failure value.
    pub fn fail(mut self, failure: F) {
        self.respond.settle(MessageSegment::Failure, &failure);
    }

    pub(crate) fn reject(mut self, code: ErrorCode) {
        self.respond.reject(code);
    }
}

impl<E: ValidatedEncode, I: ValidatedEncode, F: ValidatedEncode> StreamResponder<(), E, I, F> {
    /// Starts a stream with a unit start value and returns the item writer.
    pub fn items(self) -> ItemWriter<E, I, F> {
        self.start(())
    }
}

/// Writes streamed response items after receiving credit.
pub struct ItemWriter<E, I, F = std::convert::Infallible> {
    respond: Respond,
    response: Writer,
    failure: Option<Error>,
    values: TypeMarker<(E, I, F)>,
}

impl<E: ValidatedEncode, I: ValidatedEncode, F: ValidatedEncode> ItemWriter<E, I, F> {
    #[must_use]
    /// Returns whether the sender canceled the message.
    pub fn is_finished(&self) -> bool {
        self.respond.is_finished()
    }

    /// Sends one item when stream credit is available.
    ///
    /// # Errors
    ///
    /// Returns `Invalid` if an item violates its requirements, `Encode` if encoding fails, `Rejection` if the message
    /// was canceled, and `ChannelEnd` if the channel or connection ends first.
    pub async fn send(&mut self, item: I) -> Result<(), Error> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }

        let result = self.respond.item(&item).await;

        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }

        result
    }

    /// Sends all items, waiting for credit and batching writes within available credit.
    ///
    /// # Errors
    ///
    /// Returns `Invalid` if an item violates its requirements, `Encode` if encoding fails, `Rejection` if the message
    /// was canceled, and `ChannelEnd` if the channel or connection ends first.
    pub async fn send_all<T: Borrow<I>>(
        &mut self,
        items: impl IntoIterator<Item = T>,
    ) -> Result<(), Error> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }

        let result = self.response.write_all::<I, T>(items).await;

        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }

        result
    }

    /// Ends the response stream with the provided end value.
    pub fn end(mut self, end: E) {
        self.respond.settle(MessageSegment::ResponseEnd, &end);
    }

    /// Ends the response with a declared failure value.
    pub fn fail(mut self, failure: F) {
        self.respond.settle(MessageSegment::Failure, &failure);
    }
}

impl<I: ValidatedEncode, F: ValidatedEncode> ItemWriter<(), I, F> {
    /// Ends the response stream with a unit end value.
    pub fn finish(self) {
        self.end(());
    }
}

/// Outgoing message stream. Drop before ending the stream cancels the message.
pub struct StreamWriter<E, I> {
    writer: Writer,
    ended: bool,
    values: TypeMarker<(E, I)>,
}

impl<E: ValidatedEncode, I: ValidatedEncode> StreamWriter<E, I> {
    pub(crate) fn new(exchange: &OutgoingExchange) -> Self {
        Self {
            writer: Writer::new(std::rc::Weak::clone(&exchange.channel), exchange.message),
            ended: false,
            values: PhantomData,
        }
    }

    /// Sends one item when stream credit is available.
    ///
    /// # Errors
    ///
    /// Returns `Invalid` if an item violates its requirements, `Encode` if encoding fails, `Rejection` if the message
    /// was canceled, and `ChannelEnd` if the channel or connection ends first.
    pub async fn send(&mut self, item: &I) -> Result<(), Error> {
        self.writer.item(item).await
    }

    /// Sends all items, waiting for credit and batching writes within available credit.
    ///
    /// # Errors
    ///
    /// Returns `Invalid` if an item violates its requirements, `Encode` if encoding fails, `Rejection` if the message
    /// was canceled, and `ChannelEnd` if the channel or connection ends first.
    pub async fn send_all<T: Borrow<I>>(
        &mut self,
        items: impl IntoIterator<Item = T>,
    ) -> Result<(), Error> {
        self.writer.write_all::<I, T>(items).await
    }

    /// Waits for the message to send, then ends its stream with the provided end value.
    ///
    /// # Errors
    ///
    /// Returns `Invalid` if an item violates its requirements, `Encode` if encoding fails, `Rejection` if the message
    /// was canceled, and `ChannelEnd` if the channel or connection ends first.
    pub async fn end(mut self, end: &E) -> Result<(), Error> {
        let open_result = self.writer.open().await;

        self.ended = true;
        open_result?;

        self.writer.write(MessageSegment::MessageEnd, end)
    }
}

impl<I: ValidatedEncode> StreamWriter<(), I> {
    /// Ends the message stream with a unit end value.
    ///
    /// # Errors
    ///
    /// Returns `Invalid` if an item violates its requirements, `Encode` if encoding fails, `Rejection` if the message
    /// was canceled, and `ChannelEnd` if the channel or connection ends first.
    pub async fn finish(self) -> Result<(), Error> {
        self.end(&()).await
    }
}

impl<E, I> Drop for StreamWriter<E, I> {
    fn drop(&mut self) {
        if !self.ended {
            self.writer.cancel();
        }
    }
}
