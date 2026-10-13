use std::rc::Weak;
use std::task::{Context, Poll};

use crate::binary::ValidatedEncode;
use crate::channel::message::Event;
use crate::definition::ErrorCode;
use crate::error::Error;
use crate::frame::Rejection;
use crate::keys::EntryId;
use crate::protocol::MessageSegment;

/// Supplies stream items for writing when credit is available.
pub(crate) trait ItemSource {
    /// Writes the next item using `write`, or returns `None` when exhausted.
    fn write_next(
        &mut self,
        write: &mut dyn FnMut(&dyn ValidatedEncode) -> Result<(), Error>,
    ) -> Option<Result<(), Error>>;
}

/// Channel operations required by sender- and handler-side exchanges.
pub(crate) trait ChannelIo {
    fn release(&self, id: EntryId);
    fn release_input(&self, id: EntryId);
    fn release_output(&self, id: EntryId);
    /// Polls a sent message response. Consuming the outcome releases its message reference.
    fn poll_outcome(&self, id: EntryId, context: &mut Context<'_>) -> Poll<Event>;
    fn cancel_message(&self, id: EntryId);
    /// Polls the next incoming stream event. Consuming an item updates credit and requests more when needed.
    fn poll_event(&self, id: EntryId, context: &mut Context<'_>) -> Poll<Event>;
    /// Writes the next outgoing message-stream item after credit becomes available.
    fn poll_stream_item(
        &self,
        id: EntryId,
        value: &dyn ValidatedEncode,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), Error>>;

    /// Writes the next response-stream item for received message `id` after credit becomes available.
    fn poll_response_item(
        &self,
        id: EntryId,
        value: &dyn ValidatedEncode,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), Error>>;

    fn poll_available(&self, id: EntryId, context: &mut Context<'_>) -> Poll<Result<(), Error>>;
    fn poll_open(&self, id: EntryId, context: &mut Context<'_>) -> Poll<Result<(), Error>>;
    fn fail_input(&self, id: EntryId, rejection: Rejection);
    fn write_stream(
        &self,
        id: EntryId,
        segment: MessageSegment,
        value: &dyn ValidatedEncode,
    ) -> Result<(), Error>;

    fn write_items(&self, id: EntryId, items: &mut dyn ItemSource) -> Result<(), Error>;
    fn respond(
        &self,
        id: EntryId,
        segment: MessageSegment,
        value: &dyn ValidatedEncode,
    ) -> Result<(), Error>;

    /// Writes the final response segment and releases the responder message reference.
    fn settle(&self, id: EntryId, segment: MessageSegment, value: &dyn ValidatedEncode);
    /// Rejects the message and releases the responder message reference.
    fn reject(&self, id: EntryId, rejection: Rejection);
    fn is_finished(&self, id: EntryId) -> bool;
}

/// Controls whether dropping a message reference releases its input or output stream.
pub(crate) enum Release {
    Message,
    Input,
    Output,
}

/// Binds a message table entry to its channel and holds a counted message reference.
pub(crate) struct MessageRef {
    channel: Weak<dyn ChannelIo>,
    pub(crate) id: EntryId,
    releases: Option<Release>,
}

impl MessageRef {
    /// Creates a message reference whose count was incremented during exchange creation.
    pub(crate) fn new(channel: Weak<dyn ChannelIo>, id: EntryId, releases: Release) -> Self {
        Self {
            channel,
            id,
            releases: Some(releases),
        }
    }

    /// Disables drop release after the channel releases this reference.
    pub(crate) fn release(&mut self) {
        self.releases = None;
    }

    pub(crate) fn poll<T>(
        &self,
        poll: impl FnOnce(&dyn ChannelIo, EntryId) -> Poll<T>,
        on_error: impl FnOnce(Error) -> T,
    ) -> Poll<T> {
        match self.channel.upgrade() {
            Some(channel) => poll(&*channel, self.id),
            None => Poll::Ready(on_error(Error::ChannelEnd(ErrorCode::ClosedChannel))),
        }
    }

    pub(crate) fn with(&self, act: impl FnOnce(&dyn ChannelIo, EntryId)) {
        if let Some(channel) = self.channel.upgrade() {
            act(&*channel, self.id);
        }
    }

    pub(crate) fn try_with<T>(
        &self,
        act: impl FnOnce(&dyn ChannelIo, EntryId) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let channel = self
            .channel
            .upgrade()
            .ok_or(Error::ChannelEnd(ErrorCode::ClosedChannel))?;

        act(&*channel, self.id)
    }

    pub(crate) fn query(
        &self,
        query: impl FnOnce(&dyn ChannelIo, EntryId) -> bool,
    ) -> Option<bool> {
        self.channel
            .upgrade()
            .map(|channel| query(&*channel, self.id))
    }
}

impl Drop for MessageRef {
    fn drop(&mut self) {
        let Some(releases) = &self.releases else {
            return;
        };

        self.with(|channel, id| match releases {
            Release::Message => channel.release(id),
            Release::Input => channel.release_input(id),
            Release::Output => channel.release_output(id),
        });
    }
}

/// Channel and entry ID of a locally sent message.
pub struct OutgoingExchange {
    pub(crate) channel: Weak<dyn ChannelIo>,
    pub(crate) message: EntryId,
}

/// Channel and entry ID of a locally received message.
pub struct IncomingExchange {
    pub(crate) channel: Weak<dyn ChannelIo>,
    pub(crate) message: EntryId,
}
