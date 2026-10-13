mod handshake;
mod members;
mod message;
mod migration;
mod role;
mod tags;
mod types;

pub(crate) use handshake::HandshakeClass;
pub(crate) use message::MessageClass;
pub(crate) use migration::{MigrationClass, NEWER, OLDER};
pub(crate) use role::RoleDeclarationBuilder;
pub(crate) use tags::{FIELD_TAGS, MEMBER_TAGS, TAG, TagExpressionContainer};
pub(crate) use types::TypeDeclarationBuilder;

const BODY: &str = "body";
pub(crate) const BODY_KEY: &str = "BODY";
const FIELDS: &str = "Fields";
const INIT: &str = "Init";
const VALUE: &str = "value";
