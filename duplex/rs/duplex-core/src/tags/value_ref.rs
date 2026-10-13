use super::{FieldAccess, FieldTagContainer, Tag};

/// Dynamically typed field value returned by [`FieldAccess::field`].
#[derive(Clone)]
pub enum ValueRef<'a> {
    /// Null value or absent optional field.
    Null,
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Integer(i128),
    /// Floating-point value.
    Float(f64),
    /// Character value.
    Char(char),
    /// Borrowed string.
    Text(&'a str),
    /// Byte slice borrowed from the input.
    Bytes(&'a [u8]),
    /// List of values.
    List(Vec<ValueRef<'a>>),
    /// Record name-value entries.
    Record(Vec<(&'a str, ValueRef<'a>)>),
    /// Struct supporting [`FieldAccess`].
    Struct(&'a dyn FieldAccess),
    /// Enum member.
    Member {
        /// Declared member name.
        name: &'static str,
        /// Numeric member value.
        value: u32,
        /// Member metadata tags.
        tags: &'static [Tag],
    },
    /// Union variant with metadata and field values in declaration order.
    ///
    /// Single-field variants contain one unnamed value.
    Variant {
        /// Declared variant name.
        name: &'static str,
        /// Variant metadata tags.
        tags: &'static [Tag],
        /// Variant fields and metadata tags.
        fields: &'static [FieldTagContainer],
        /// Variant field values.
        values: Vec<ValueRef<'a>>,
    },
}

impl std::fmt::Debug for ValueRef<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => formatter.write_str("Null"),
            Self::Bool(value) => formatter.debug_tuple("Bool").field(value).finish(),
            Self::Integer(value) => formatter.debug_tuple("Integer").field(value).finish(),
            Self::Float(value) => formatter.debug_tuple("Float").field(value).finish(),
            Self::Char(value) => formatter.debug_tuple("Char").field(value).finish(),
            Self::Text(value) => formatter.debug_tuple("Text").field(value).finish(),
            Self::Bytes(value) => formatter.debug_tuple("Bytes").field(value).finish(),
            Self::List(items) => formatter.debug_tuple("List").field(items).finish(),
            Self::Record(entries) => formatter.debug_tuple("Record").field(entries).finish(),
            Self::Struct(value) => {
                let mut map = formatter.debug_map();

                for field in value.fields() {
                    map.entry(&field.name, &value.field(field.name));
                }

                map.finish()
            }
            Self::Member { name, value, .. } => formatter
                .debug_tuple("Member")
                .field(name)
                .field(value)
                .finish(),
            Self::Variant { name, values, .. } => formatter
                .debug_tuple("Variant")
                .field(name)
                .field(values)
                .finish(),
        }
    }
}

impl<'a> ValueRef<'a> {
    /// Returns an integer or enum member numeric value, or `None` for other variants.
    #[must_use]
    pub fn integer(&self) -> Option<i128> {
        match self {
            Self::Integer(integer) => Some(*integer),
            Self::Member { value, .. } => Some(i128::from(*value)),
            _ => None,
        }
    }

    /// Returns the string, or `None` if this variant is not a string.
    #[must_use]
    pub fn text(&self) -> Option<&'a str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }
}
