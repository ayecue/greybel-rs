//! Parsing, validation, versioning, and planning for duplex definitions.

mod constraints;
mod error;
mod fingerprint;
mod model;
mod names;
mod pattern;
mod plan;
mod syntax;
mod versions;

pub use error::DefinitionError;
pub use model::{
    Constant, CoreUse, Definition, DocumentationRef, Field, FieldId, Flavour, Handshake,
    HealthCheckSetting, Length, Literal, LiteralError, Member, MemberId, Message, NumberKind,
    Payload, Peer, Protocol, Range, Response, ResponseBody, Role, Security, SecurityLevel, Span,
    Status, Stream, StreamField, StringRequirementContainer, Tag, TagDeclaration, TagList,
    TagPlace, TypeDeclaration, TypeKind, TypeRef, Value, Variant, VariantId, Version,
};
pub use names::{MessageSegment, Name, Symbol};
pub use pattern::{PatternError, PatternTable};
pub use plan::{
    Bit, BlockPlan, ConstantPlan, Direction, Encoding, FieldPlan, FieldValue, FieldsPlan,
    MemberPlan, MessageMigration, MessagePlan, MigrationError, MigrationPlan, PayloadPlan, Plan,
    Presence, ProtocolPlan, ResponseBodyPlan, ResponsePlan, Rewrite, RolePlan, StreamPlan,
    TypeContent, TypePlan, VariantPlan, VersionPlan,
};
pub use versions::{
    AdditionTable, DefinitionVersion, FingerprintTable, FolderFailure, MinorVersionTable,
    VersionHistory, VersionsError,
};
