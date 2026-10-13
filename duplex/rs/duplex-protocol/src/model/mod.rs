mod definition;
mod docs;
mod ids;
mod literal;
mod protocol;
mod requirements;
mod size;
mod tags;
mod types;
mod version;

pub use definition::{CoreUse, Definition};
pub use docs::{DocumentationRef, Span};
pub use ids::{FieldId, MemberId, VariantId};
pub use literal::{Constant, Literal, LiteralError, Value};
pub use protocol::{
    Flavour, Handshake, HealthCheckSetting, Message, Peer, Protocol, Response, ResponseBody, Role,
    Security, SecurityLevel, Stream, StreamField,
};
pub use requirements::{Length, Range, StringRequirementContainer};
pub use tags::{Tag, TagDeclaration, TagList, TagPlace};
pub use types::{Field, Member, NumberKind, Payload, TypeDeclaration, TypeKind, TypeRef, Variant};
pub use version::{Status, Version};

/// Generates a public `span()` method for each declaration's source position.
macro_rules! declared_at {
    ($($declaration:ty),* $(,)?) => {
        $(
            impl $declaration {
                /// Returns declaration source position.
                #[must_use]
                pub fn span(&self) -> Span {
                    self.span
                }
            }
        )*
    };
}

declared_at!(
    Constant,
    Field,
    Member,
    Message,
    Peer,
    Protocol,
    StreamField,
    TagDeclaration,
    TypeDeclaration,
    Variant,
);
