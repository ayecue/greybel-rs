use crate::model::{Field, Message, Peer, Protocol};

const PEERS_SUFFIX: &str = "Peer";
const HANDSHAKE_SUFFIX: &str = "Handshake";
const ROUTER_SUFFIX: &str = "Router";
const CHANNEL_SUFFIX: &str = "Channel";
const NOTIFICATION_SUFFIX: &str = "Notification";
const HANDLER_PREFIX: &str = "on";
const SEND_PREFIX: &str = "send";
const MIGRATION_SUFFIX: &str = "Migration";
const MIGRATE_PREFIX: &str = "migrate";

/// Value or structural segment in a message exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageSegment {
    /// Initial message body, which also starts the sender message stream when present.
    Message,
    /// Sender message-stream item.
    MessageItem,
    /// Sender message-stream end value.
    MessageEnd,
    /// Streamed response start value.
    ResponseStart,
    /// Streamed response item.
    ResponseItem,
    /// Successful response end value.
    ResponseEnd,
    /// Declared failure value.
    Failure,
}

impl MessageSegment {
    /// Returns every message segment in exchange order.
    pub const ALL: [Self; 7] = [
        Self::Message,
        Self::MessageItem,
        Self::MessageEnd,
        Self::ResponseStart,
        Self::ResponseItem,
        Self::ResponseEnd,
        Self::Failure,
    ];

    #[must_use]
    pub fn pascal(self) -> &'static str {
        match self {
            Self::Message => "Message",
            Self::MessageItem => "MessageItem",
            Self::MessageEnd => "MessageEnd",
            Self::ResponseStart => "ResponseStart",
            Self::ResponseItem => "ResponseItem",
            Self::ResponseEnd => "ResponseEnd",
            Self::Failure => "Failure",
        }
    }

    #[must_use]
    pub fn is_response(self) -> bool {
        matches!(
            self,
            Self::ResponseStart | Self::ResponseItem | Self::ResponseEnd | Self::Failure
        )
    }
}

/// Declaration name decomposed into words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name(Vec<String>);

impl Name {
    fn of_pascal(text: &str) -> Self {
        let mut words: Vec<String> = Vec::new();

        for char in text.chars() {
            match words.last_mut() {
                Some(word) if !char.is_ascii_uppercase() => word.push(char),
                _ => words.push(char.to_ascii_lowercase().to_string()),
            }
        }

        Self(words)
    }

    fn of_snake(text: &str) -> Self {
        Self(
            text.split('_')
                .filter(|word| !word.is_empty())
                .map(str::to_owned)
                .collect(),
        )
    }

    fn with_prefix(mut self, word: &str) -> Self {
        self.0.insert(0, word.to_owned());

        self
    }

    fn with_suffix(mut self, word: &str) -> Self {
        self.0.push(word.to_ascii_lowercase());

        self
    }

    fn extend_pascal(mut self, pascal: &str) -> Self {
        self.0.extend(Self::of_pascal(pascal).0);

        self
    }

    #[must_use]
    pub fn pascal(&self) -> String {
        self.0.iter().map(|word| Self::capitalize(word)).collect()
    }

    #[must_use]
    pub fn camel(&self) -> String {
        let mut words = self.0.iter();
        let first = words.next().cloned().unwrap_or_default();

        words.fold(first, |name, word| name + &Self::capitalize(word))
    }

    #[must_use]
    pub fn snake(&self) -> String {
        self.0.join("_")
    }

    fn capitalize(word: &str) -> String {
        let mut chars = word.chars();

        chars.next().map_or_else(String::new, |first| {
            first.to_ascii_uppercase().to_string() + chars.as_str()
        })
    }
}

/// Definition element used for generated naming.
#[derive(Debug, Clone, Copy)]
pub enum Symbol<'d> {
    /// Declared protocol, peer, message, type, variant, or member.
    Declared(&'d str),
    /// Declaration containing both protocol peers.
    Peers(&'d Protocol),
    /// Protocol handshake declaration.
    Handshake(&'d Protocol),
    /// Peer router declaration.
    Router(&'d Peer),
    /// Peer channel declaration.
    Channel(&'d Peer),
    /// Enum of notifications handled by a peer.
    Notification(&'d Peer),
    /// Message handler method.
    Handler(&'d Message),
    /// Message-sending channel method.
    Send(&'d Message),
    /// Field declared in `snake_case`.
    Field(&'d Field),
    /// Major-version protocol migration type.
    Migration(&'d Protocol, u16),
    /// Message-segment migration method.
    Migrate(&'d Message, MessageSegment),
}

impl Symbol<'_> {
    #[must_use]
    pub fn name(self) -> Name {
        match self {
            Self::Declared(name) => Name::of_pascal(name),
            Self::Peers(protocol) => Name::of_pascal(&protocol.name).with_suffix(PEERS_SUFFIX),
            Self::Handshake(protocol) => {
                Name::of_pascal(&protocol.name).with_suffix(HANDSHAKE_SUFFIX)
            }
            Self::Router(peer) => Name::of_pascal(&peer.name).with_suffix(ROUTER_SUFFIX),
            Self::Channel(peer) => Name::of_pascal(&peer.name).with_suffix(CHANNEL_SUFFIX),
            Self::Notification(peer) => {
                Name::of_pascal(&peer.name).with_suffix(NOTIFICATION_SUFFIX)
            }
            Self::Handler(message) => Name::of_pascal(&message.name).with_prefix(HANDLER_PREFIX),
            Self::Send(message) => Name::of_pascal(&message.name).with_prefix(SEND_PREFIX),
            Self::Field(field) => Name::of_snake(&field.name),
            Self::Migration(protocol, major) => Name::of_pascal(&protocol.name)
                .with_suffix(MIGRATION_SUFFIX)
                .with_suffix(&major.to_string()),
            Self::Migrate(message, MessageSegment::Message) => {
                Name::of_pascal(&message.name).with_prefix(MIGRATE_PREFIX)
            }
            Self::Migrate(message, segment) => Name::of_pascal(&message.name)
                .with_prefix(MIGRATE_PREFIX)
                .extend_pascal(segment.pascal()),
        }
    }

    #[must_use]
    pub fn pascal(self) -> String {
        self.name().pascal()
    }
}
