use crate::model::{
    Definition, Length, NumberKind, Range, StringRequirementContainer, TypeKind, TypeRef,
};

/// Binary encoding for a type reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Encoding<'d> {
    /// Encodes as 0 or 1.
    Bool,
    /// Varuint-encoded code point.
    Char,
    /// Varuint byte length and UTF-8 bytes, subject to string requirements.
    String(&'d StringRequirementContainer),
    /// Varuint length and bytes, subject to the length requirement.
    Bytes(Length),
    /// Bounded numeric encoding: varuint, little-endian float, or fixed-width little-endian integer.
    Number(NumberKind, &'d Range),
    /// Varuint item count and encoded items.
    List(Box<Self>, Length),
    /// Varuint entry count, string keys, and encoded values.
    Record(Box<Self>, Length),
    /// Presence byte and, for non-null values, the encoded value.
    Nullable(Box<Self>),
    /// Flag bytes and struct fields.
    Struct(&'d str),
    /// Varuint variant index and payload.
    Union(&'d str),
    /// Encoded enum member numeric value.
    Enumeration(&'d str),
}

impl<'d> Encoding<'d> {
    pub(crate) fn of(definition: &'d Definition, ty: &'d TypeRef) -> Self {
        match ty {
            TypeRef::Bool => Self::Bool,
            TypeRef::Char => Self::Char,
            TypeRef::String(requirements) => Self::String(requirements),
            TypeRef::Bytes(length) => Self::Bytes(*length),
            TypeRef::Number(kind, bounds) => Self::Number(*kind, bounds),
            TypeRef::List(item, length) => {
                Self::List(Box::new(Self::of(definition, item)), *length)
            }
            TypeRef::Record(value, length) => {
                Self::Record(Box::new(Self::of(definition, value)), *length)
            }
            TypeRef::Nullable(inner) => Self::Nullable(Box::new(Self::of(definition, inner))),
            TypeRef::Declared(name) => match definition.declaration(name).map(|it| &it.kind) {
                Some(TypeKind::Union(_)) => Self::Union(name),
                Some(TypeKind::Enumeration(_)) => Self::Enumeration(name),
                Some(TypeKind::Struct(_)) | None => Self::Struct(name),
            },
        }
    }

    #[must_use]
    pub fn non_null(&self) -> &Self {
        match self {
            Self::Nullable(inner) => inner,
            _ => self,
        }
    }

    #[must_use]
    pub fn has_requirements(&self) -> bool {
        match self {
            Self::Bool | Self::Char | Self::Struct(_) | Self::Union(_) | Self::Enumeration(_) => {
                false
            }
            Self::String(requirements) => !requirements.is_any(),
            Self::Bytes(length) => !length.is_any(),
            Self::Number(_, bounds) => !bounds.is_any(),
            Self::List(item, length) | Self::Record(item, length) => {
                !length.is_any() || item.has_requirements()
            }
            Self::Nullable(inner) => inner.has_requirements(),
        }
    }

    #[must_use]
    pub fn declared_name(&self) -> Option<&'d str> {
        match self {
            Self::Struct(name) | Self::Union(name) | Self::Enumeration(name) => Some(name),
            _ => None,
        }
    }
}
