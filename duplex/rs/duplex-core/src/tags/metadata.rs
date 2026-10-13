use super::{FieldTagContainer, Tag, ValueRef};

/// Generated message, struct, union, or enumeration metadata.
pub trait Metadata {
    /// Tags declared on this item.
    const TAGS: &'static [Tag];
    /// Fields and tags in declaration order, or empty for unions and enumerations.
    const FIELDS: &'static [FieldTagContainer] = &[];
}

/// Accesses generated message or struct fields by name.
pub trait FieldAccess {
    /// Returns fields and tags in declaration order.
    fn fields(&self) -> &'static [FieldTagContainer];

    /// Returns the value of field `name`.
    fn field(&self, name: &str) -> Option<ValueRef<'_>>;
}
