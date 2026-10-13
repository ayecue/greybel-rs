use super::{
    DocumentationRef, FieldId, Length, Literal, MemberId, Range, Span, StringRequirementContainer,
    TagList, VariantId,
};

/// Struct, union, or enumeration declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDeclaration {
    pub docs: DocumentationRef,
    pub tags: TagList,
    pub name: String,
    /// Declaration kind and contents.
    pub kind: TypeKind,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    /// Struct fields.
    Struct(Vec<Field>),
    /// Union variants.
    Union(Vec<Variant>),
    /// Enumeration members.
    Enumeration(Vec<Member>),
}

/// Struct, union variant, or message field declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Declaration-order field ID.
    pub id: FieldId,
    pub docs: DocumentationRef,
    pub tags: TagList,
    pub name: String,
    /// Whether writers may omit this field.
    pub optional: bool,
    /// Whether struct fields are inline in the containing encoding.
    pub flatten: bool,
    pub ty: TypeRef,
    /// Default used when an older minor version omits this field.
    pub default: Option<Literal>,
    pub(crate) span: Span,
}

/// Union variant declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    /// Declaration-order variant ID.
    pub id: VariantId,
    pub docs: DocumentationRef,
    pub tags: TagList,
    pub name: String,
    /// Variant payload.
    pub payload: Payload,
    pub(crate) span: Span,
}

/// Union variant payload.
#[derive(Debug, Clone, PartialEq)]
pub enum Payload {
    /// Variant has no payload.
    Unit,
    /// One typed value.
    Value(TypeRef),
    Fields(Vec<Field>),
}

/// Enumeration member declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    /// Declaration-order member ID.
    pub id: MemberId,
    pub docs: DocumentationRef,
    pub tags: TagList,
    pub name: String,
    /// Optional explicit member numeric value.
    pub value: Option<u32>,
    pub(crate) span: Span,
}

/// Built-in or declared type reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeRef {
    Bool,
    /// Unicode character.
    Char,
    /// UTF-8 string with length and pattern constraints.
    String(StringRequirementContainer),
    /// Byte sequence with length requirement.
    Bytes(Length),
    /// Numeric encoding with optional bounds.
    Number(NumberKind, Range),
    /// Sequence with item type and length requirement.
    List(Box<TypeRef>, Length),
    /// String-keyed map with entry-count requirement.
    Record(Box<TypeRef>, Length),
    /// Nullable value.
    Nullable(Box<TypeRef>),
    /// Named declared-type reference.
    Declared(String),
}

impl TypeRef {
    #[must_use]
    pub fn non_null(&self) -> &Self {
        match self {
            Self::Nullable(inner) => inner,
            _ => self,
        }
    }

    #[must_use]
    pub fn is_nullable(&self) -> bool {
        matches!(self, Self::Nullable(_))
    }

    #[must_use]
    pub fn is_same_type(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool, Self::Bool)
            | (Self::Char, Self::Char)
            | (Self::String(_), Self::String(_))
            | (Self::Bytes(_), Self::Bytes(_)) => true,
            (Self::Number(kind, _), Self::Number(other, _)) => kind == other,
            (Self::List(item, _), Self::List(other, _))
            | (Self::Record(item, _), Self::Record(other, _))
            | (Self::Nullable(item), Self::Nullable(other)) => item.is_same_type(other),
            (Self::Declared(name), Self::Declared(other)) => name == other,
            _ => false,
        }
    }

    #[must_use]
    pub fn has_requirements(&self) -> bool {
        match self {
            Self::Bool | Self::Char | Self::Declared(_) => false,
            Self::String(requirements) => !requirements.is_any(),
            Self::Bytes(length) => !length.is_any(),
            Self::Number(_, bounds) => !bounds.is_any(),
            Self::List(item, length) | Self::Record(item, length) => {
                !length.is_any() || item.has_requirements()
            }
            Self::Nullable(inner) => inner.has_requirements(),
        }
    }
}

/// Definition-file numeric types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumberKind {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    /// Fixed-width unsigned 32-bit integer.
    Fixed32,
    /// Fixed-width unsigned 64-bit integer.
    Fixed64,
}

impl NumberKind {
    /// Returns numeric kinds in declaration order.
    pub const ALL: [Self; 12] = [
        Self::U8,
        Self::U16,
        Self::U32,
        Self::U64,
        Self::I8,
        Self::I16,
        Self::I32,
        Self::I64,
        Self::F32,
        Self::F64,
        Self::Fixed32,
        Self::Fixed64,
    ];

    #[must_use]
    pub fn keyword(self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::Fixed32 => "fixed32",
            Self::Fixed64 => "fixed64",
        }
    }

    #[must_use]
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.keyword() == keyword)
    }

    #[must_use]
    pub fn is_64_bit_integer(self) -> bool {
        matches!(self, Self::U64 | Self::I64 | Self::Fixed64)
    }

    #[must_use]
    pub fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }
}
