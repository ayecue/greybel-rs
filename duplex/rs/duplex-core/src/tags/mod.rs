//! Metadata tags for protocols, messages, types, fields, variants, and enum members.
//! Generated code represents `@name(value)` annotations with these values. Tags do not change binary layout.

mod as_value_ref;
mod field_tag_container;
mod metadata;
mod tag;
mod value_ref;

pub use as_value_ref::AsValueRef;
pub use field_tag_container::FieldTagContainer;
pub use metadata::{FieldAccess, Metadata};
pub use tag::{Tag, TagValue};
pub use value_ref::ValueRef;
