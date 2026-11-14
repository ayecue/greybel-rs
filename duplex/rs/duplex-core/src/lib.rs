//! Typed two-peer protocol channels that batch messages into binary transport frames.
//! `include_protocol!` generates a protocol from a `.protocol` definition.

#![warn(missing_docs)]
mod binary;
mod channel;
mod connection_id;
mod definition;
mod dispatch;
mod driver;
mod error;
mod error_code;
mod frame;
mod hub;
mod keys;
mod migration;
mod protocol;
#[cfg(feature = "security")]
mod security;
mod tags;
mod transport;

pub use binary::{
    BinaryDecode, BinaryEncode, BinaryReader, BinaryWriter, BorrowDecode, Bytes, Fixed32, Fixed64,
    Pattern, Validatable, ValidatedDecode, ValidatedEncode, Validation, Varuint,
};
pub use channel::{
    Bundle, Channel, ChannelOptions, CloseFuture, ConnectionActive, ConnectionEnd,
    ConnectionFailure, ConnectionInit, ConnectionState, DEFAULT_MAX_MESSAGE, DEFAULT_STREAM_WINDOW,
    Health, HealthCheck, HealthChecks, HookId, IncomingStream, ItemWriter, Limits, NextMessage,
    NextNotification, PendingConnection, PendingResponse, Responder, ResponseStream, SendOptions,
    StreamResponder, StreamWriter, TransportFlavour,
};
pub use connection_id::ConnectionId;
pub use definition::{CORE, ErrorCode, Requirement, Role, Version, Violation};
pub use dispatch::{Dispatch, ResponderOf};
pub use driver::{
    BatchStream, Capabilities, Driver, InboundBatch, InboundBufferManager, OutboundBatch, TimerOf,
};
pub use duplex_macros::{include_migration, include_protocol, include_types};
pub use error::{DecodeFailure, EncodeFailure, Error};
pub use hub::Hub;
pub use migration::Migration;
pub use protocol::{
    Exchange, Message, MessageRequirementTable, MessageSegment, NoResponse, NotificationMessage,
    Peer, Protocol, SecurityLevel, StreamExchange, StreamResponse, ValueResponse, VersionInfo,
    VersionStatus,
};
#[cfg(feature = "security")]
pub use security::{EntropyUnavailable, Security, SecurityRole};
pub use tags::{AsValueRef, FieldAccess, FieldTagContainer, Metadata, Tag, TagValue, ValueRef};
pub use transport::{Port, PortReceiver, PortSender, Relay, Transport, WeakTransport};
