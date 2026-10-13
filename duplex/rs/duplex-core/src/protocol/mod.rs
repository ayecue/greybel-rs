mod exchange;

use std::fmt::Debug;

pub use exchange::{Exchange, NoResponse, StreamExchange, StreamResponse, ValueResponse};

use crate::binary::{
    BinaryDecode, BinaryEncode, BinaryReader, BinaryWriter, BorrowDecode, Validatable, Validation,
};
use crate::channel::TransportFlavour;
use crate::definition::{Role, Version};
use crate::driver::Capabilities;
use crate::error::Error;

#[doc(hidden)]
pub trait Protocol: Sized + 'static {
    #[doc(hidden)]
    const CHANNEL: &'static str;
    #[doc(hidden)]
    const CHANNEL_ID: u32;
    const VERSION: Version;
    #[doc(hidden)]
    const VERSIONS: &'static [VersionInfo];
    #[doc(hidden)]
    const FLAVOURS: &'static [TransportFlavour];
    #[doc(hidden)]
    const HANDSHAKE: bool;
    #[doc(hidden)]
    const SECURITY: SecurityLevel;
    #[doc(hidden)]
    const SIGNED: bool;
    #[doc(hidden)]
    const REQUIRES: Capabilities;
    type Open: BinaryEncode + BinaryDecode + 'static;
    type Ack: BinaryEncode + BinaryDecode + Clone + 'static;
    type Refusal: BinaryEncode + BinaryDecode + Clone + Debug + 'static;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
/// Protocol transport security mode.
pub enum SecurityLevel {
    #[default]
    /// Sends unencrypted, unauthenticated batches.
    None,
    /// Encrypts batches without authenticating peer identities.
    Encryption,
    /// Encrypts batches and authenticates the remote peer.
    Authentication,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionInfo {
    pub fingerprint: u32,
    pub status: VersionStatus,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionStatus {
    Current,
    Deprecation(&'static str),
    Unavailable,
}

#[doc(hidden)]
pub trait Peer: 'static {
    type Protocol: Protocol;
    type Remote: Peer<Protocol = Self::Protocol, Remote = Self>;
    const ROLE: Role;
    type OpenPayload: BinaryEncode + 'static;
    /// Outgoing message frame requirements indexed by message type ID.
    #[doc(hidden)]
    const SEND_REQUIREMENTS: &'static [&'static MessageRequirementTable] = &[];
    /// Outgoing message timeouts indexed by message type ID.
    #[doc(hidden)]
    const TIMEOUTS: &'static [Option<std::time::Duration>] = &[];
}

#[doc(hidden)]
pub trait Message: BinaryEncode + Validatable {
    #[doc(hidden)]
    const ID: u32;
    #[doc(hidden)]
    const SINCE: u16;
    #[doc(hidden)]
    const TIMEOUT: Option<std::time::Duration> = None;
    #[doc(hidden)]
    const REQUIREMENTS: &'static MessageRequirementTable = &MessageRequirementTable::NONE;
    type Sender: Peer;
    type Exchange: Exchange;

    #[doc(hidden)]
    fn transcode(reader: &mut BinaryReader<'_>, writer: &mut BinaryWriter) -> Result<(), Error>;
}

/// Response-less message whose handlers receive input-borrowed values.
pub trait NotificationMessage: Message<Exchange = NoResponse> {
    /// Handler input type borrowed from the message; `into_owned` converts it to the owned type.
    type Ref<'a>: BorrowDecode<'a>;
}

/// Message frame and stream requirements applied in addition to field validation.
#[doc(hidden)]
#[derive(Debug)]
pub struct MessageRequirementTable {
    /// Maximum client-sent message frame body bytes.
    pub limit: u32,
    /// Maximum declared encoded item size in the message stream.
    pub message_item_size: Option<u32>,
    /// Maximum declared encoded item size in the response stream.
    pub response_item_size: Option<u32>,
    /// Item-count requirements for the message stream.
    pub message_items: Validation,
    /// Item-count requirements for the response stream.
    pub response_items: Validation,
    pub message_item: Validation,
    pub message_end: Validation,
    pub response_start: Validation,
    pub response_item: Validation,
    pub response_end: Validation,
    pub failure: Validation,
}

impl MessageRequirementTable {
    /// Default client-sent message frame body limit when the protocol declares none.
    pub const DEFAULT_LIMIT: u32 = 64 * 1024;
    pub const NONE: Self = Self {
        limit: Self::DEFAULT_LIMIT,
        message_item_size: None,
        response_item_size: None,
        message_items: Validation::Any,
        response_items: Validation::Any,
        message_item: Validation::Any,
        message_end: Validation::Any,
        response_start: Validation::Any,
        response_item: Validation::Any,
        response_end: Validation::Any,
        failure: Validation::Any,
    };

    #[must_use]
    pub fn of(&self, segment: MessageSegment) -> &Validation {
        match segment {
            MessageSegment::Message => &Validation::Any,
            MessageSegment::MessageItem => &self.message_item,
            MessageSegment::MessageEnd => &self.message_end,
            MessageSegment::ResponseStart => &self.response_start,
            MessageSegment::ResponseItem => &self.response_item,
            MessageSegment::ResponseEnd => &self.response_end,
            MessageSegment::Failure => &self.failure,
        }
    }

    pub(crate) fn items_of(&self, segment: MessageSegment) -> &Validation {
        match segment {
            MessageSegment::MessageItem => &self.message_items,
            MessageSegment::ResponseItem => &self.response_items,
            _ => &Validation::Any,
        }
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageSegment {
    Message,
    MessageItem,
    MessageEnd,
    ResponseStart,
    ResponseItem,
    ResponseEnd,
    Failure,
}

impl MessageSegment {
    #[must_use]
    pub fn is_response(self) -> bool {
        matches!(
            self,
            Self::ResponseStart | Self::ResponseItem | Self::ResponseEnd | Self::Failure
        )
    }

    pub(crate) fn frame_kind(self) -> crate::definition::FrameKind {
        use crate::definition::FrameKind;

        match self {
            Self::Message => FrameKind::Message,
            Self::MessageItem => FrameKind::MessageItem,
            Self::MessageEnd => FrameKind::MessageEnd,
            Self::ResponseStart => FrameKind::ResponseStart,
            Self::ResponseItem => FrameKind::ResponseItem,
            Self::ResponseEnd => FrameKind::ResponseEnd,
            Self::Failure => FrameKind::Failure,
        }
    }

    pub(crate) fn is_response_end(self) -> bool {
        matches!(self, Self::ResponseEnd | Self::Failure)
    }

    #[must_use]
    pub(crate) fn should_migrate_up(self, receiver: Role) -> bool {
        (receiver == Role::Provider) != self.is_response()
    }
}
