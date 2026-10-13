mod body;

pub use body::ResponseBody;

/// Server-to-client batch delivery mode.
pub trait Downstream: 'static {
    /// Transport name for selecting protocol flavor settings.
    const TRANSPORT: &'static str;
    #[doc(hidden)]
    const MODE: Mode;
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    LongPoll,
    EventStream,
    ByteStream,
}

/// Delivers batches with long-polled GET requests and immediate POST responses.
#[derive(Debug, Clone, Copy)]
pub struct LongPoll;

/// Delivers server-sent events over persistent GET `/events`, using sequence numbers as event IDs and base64 batch
/// payloads.
#[derive(Debug, Clone, Copy)]
pub struct EventStream;

/// Delivers length-prefixed records over persistent GET `/stream`, with empty records as heartbeats.
#[derive(Debug, Clone, Copy)]
pub struct ByteStream;

impl Downstream for LongPoll {
    const MODE: Mode = Mode::LongPoll;
    const TRANSPORT: &'static str = "http";
}

impl Downstream for EventStream {
    const MODE: Mode = Mode::EventStream;
    const TRANSPORT: &'static str = "sse";
}

impl Downstream for ByteStream {
    const MODE: Mode = Mode::ByteStream;
    const TRANSPORT: &'static str = "http_stream";
}
