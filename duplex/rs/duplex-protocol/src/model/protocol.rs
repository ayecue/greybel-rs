use super::{DocumentationRef, Field, Length, Span, TagList, TypeRef};
use crate::fingerprint::Fnv;

/// Protocol participant role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Peer initiating the connection.
    Client,
    /// Peer accepting the connection.
    Provider,
}

impl Role {
    /// Returns both roles in identifier order.
    pub const ALL: [Self; 2] = [Self::Client, Self::Provider];

    #[must_use]
    pub fn id(self) -> u8 {
        match self {
            Self::Client => 0,
            Self::Provider => 1,
        }
    }

    #[must_use]
    pub fn remote(self) -> Self {
        match self {
            Self::Client => Self::Provider,
            Self::Provider => Self::Client,
        }
    }

    #[must_use]
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Client => "client",
            Self::Provider => "provider",
        }
    }

    #[must_use]
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.keyword() == keyword)
    }
}

/// Protocol declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Protocol {
    pub docs: DocumentationRef,
    pub tags: TagList,
    pub name: String,
    /// Channel name hashed into the channel ID.
    pub channel: String,
    /// Types for open, acknowledgement, and refusal payloads.
    pub handshake: Handshake,
    /// Per-transport channel settings.
    pub flavours: Vec<Flavour>,
    /// Protocol channel transport security requirements.
    pub security: Option<Security>,
    /// Default client frame limit for messages without an explicit limit.
    pub limit: Option<u32>,
    pub(crate) peers: [Peer; 2],
    pub(crate) messages: Vec<Message>,
    pub(crate) span: Span,
}

impl Protocol {
    /// Uses 64 KiB when neither message nor protocol specifies a client frame limit.
    pub const DEFAULT_LIMIT: u32 = 64 * 1024;

    #[must_use]
    pub fn limit_of(&self, message: &Message) -> u32 {
        message.limit.or(self.limit).unwrap_or(Self::DEFAULT_LIMIT)
    }

    #[must_use]
    pub fn peer(&self, role: Role) -> &Peer {
        &self.peers[usize::from(role.id())]
    }

    #[must_use]
    pub fn peers(&self) -> &[Peer; 2] {
        &self.peers
    }

    #[must_use]
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn inbound_messages(&self, role: Role) -> impl Iterator<Item = &Message> {
        self.messages
            .iter()
            .filter(move |message| message.receiver == role)
    }

    #[must_use]
    pub fn channel_id(&self) -> u32 {
        Fnv::of(self.channel.as_bytes())
    }
}

/// Protocol channel transport security requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Security {
    /// Minimum transport security level.
    pub level: SecurityLevel,
    /// Requires every batch to include the sender signature when true.
    pub signed: bool,
}

/// Transport security level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    /// Requires encrypted batches without peer identity authentication.
    Encryption,
    /// Requires encrypted batches and remote peer identity authentication.
    Authentication,
}

impl SecurityLevel {
    /// Returns security levels from least to most secure.
    pub const ALL: [Self; 2] = [Self::Encryption, Self::Authentication];

    #[must_use]
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Encryption => "encrypted",
            Self::Authentication => "authenticated",
        }
    }
}

/// Protocol-declared per-transport channel behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flavour {
    /// Driver transport name.
    pub transport: String,
    /// Optional health check configuration.
    pub health_check: Option<HealthCheckSetting>,
    /// Maximum stream items sent before the receiver grants more credit.
    pub stream_window: Option<u32>,
    /// Default handler timeout in milliseconds for messages without one.
    pub timeout: Option<u32>,
    /// Maximum encoded message or segment frame size.
    pub max_message: Option<u32>,
    pub(crate) span: Span,
}

/// Transport health check configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthCheckSetting {
    /// Disables health checks.
    Off,
    /// Enables health checks with optional timing overrides.
    On {
        /// Health check interval in milliseconds.
        interval: Option<u32>,
        /// Connection becomes unhealthy after exceeding this consecutive missed-check count.
        unhealthy_threshold: Option<u32>,
        /// Channel closes as `Timeout` after exceeding this consecutive missed-check count.
        close_threshold: Option<u32>,
    },
}

/// Channel opening payloads.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Handshake {
    /// Client channel-open payload.
    pub open: Option<TypeRef>,
    /// Provider channel-acceptance payload.
    pub ack: Option<TypeRef>,
    /// Provider channel-refusal payload.
    pub refusal: Option<TypeRef>,
}

impl Handshake {
    #[must_use]
    pub fn is_declared(&self) -> bool {
        self.open.is_some() || self.ack.is_some() || self.refusal.is_some()
    }
}

/// Protocol peer declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Peer {
    pub docs: DocumentationRef,
    pub name: String,
    /// Peer protocol role.
    pub role: Role,
    pub(crate) span: Span,
}

/// Protocol message declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub docs: DocumentationRef,
    pub tags: TagList,
    pub name: String,
    /// Previous-major message name used to pair migration steps.
    pub renames: Option<String>,
    /// Message ID within its receiver.
    pub id: u32,
    /// Receiving role.
    pub receiver: Role,
    /// Initial message fields.
    pub fields: Vec<Field>,
    /// Sender stream following the initial fields.
    pub stream: Option<StreamField>,
    /// Response definition, or `None` for a notification.
    pub response: Option<Response>,
    /// Handler timeout in milliseconds and maximum inter-item delay for streams.
    pub timeout: Option<u32>,
    /// Maximum encoded frame size for the message, stream items, or response segments.
    pub limit: Option<u32>,
    pub(crate) span: Span,
}

impl Message {
    #[must_use]
    pub fn sender(&self) -> Role {
        self.receiver.remote()
    }

    #[must_use]
    pub fn failure(&self) -> Option<&TypeRef> {
        self.response.as_ref()?.failure.as_ref()
    }

    pub fn types(&self) -> impl Iterator<Item = &TypeRef> {
        let stream = self.stream.iter().flat_map(|field| field.stream.types());
        let response = self.response.iter().flat_map(|response| {
            let body: Box<dyn Iterator<Item = &TypeRef>> = match &response.body {
                ResponseBody::Value(ty) => Box::new(std::iter::once(ty)),
                ResponseBody::Stream(stream) => Box::new(stream.types()),
            };

            body.chain(response.failure.iter())
        });

        self.fields
            .iter()
            .map(|field| &field.ty)
            .chain(stream)
            .chain(response)
    }
}

/// Sender stream following initial message fields.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamField {
    pub docs: DocumentationRef,
    pub name: String,
    /// Stream start, item, and end types.
    pub stream: Stream,
    pub(crate) span: Span,
}

/// Message response and declared failure.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    /// Successful response body.
    pub body: ResponseBody,
    /// Declared failure type. Streamed responses can fail after emitting items.
    pub failure: Option<TypeRef>,
}

/// Successful message response.
#[derive(Debug, Clone, PartialEq)]
pub enum ResponseBody {
    /// Single response value.
    Value(TypeRef),
    /// Streamed response.
    Stream(Stream),
}

/// Stream layout with optional start and end values, item type, and item-count requirements.
#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    /// Value sent before stream items.
    pub start: Option<TypeRef>,
    /// Value sent after stream items.
    pub end: Option<TypeRef>,
    /// Stream item type.
    pub item: TypeRef,
    /// Inclusive requirements on the stream item count.
    pub items: Length,
}

impl Stream {
    pub fn types(&self) -> impl Iterator<Item = &TypeRef> {
        self.start
            .iter()
            .chain(self.end.iter())
            .chain(std::iter::once(&self.item))
    }
}
