/// Metadata tag on a definition item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tag {
    /// Declared tag name.
    pub name: &'static str,
    /// Tag value.
    pub value: TagValue,
}

/// Tag value with its declared type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TagValue {
    /// Tag with no declared value type.
    Flag,
    /// Boolean tag value.
    Bool(bool),
    /// Integer tag value.
    Integer(i128),
    /// Floating-point tag value.
    Float(f64),
    /// String tag value.
    Text(&'static str),
    /// Character tag value.
    Char(char),
    /// Enum member tag value.
    Member {
        /// Declared member name.
        name: &'static str,
        /// Numeric member value.
        value: u32,
    },
}

impl Tag {
    /// Returns the first tag named `name` in `tags`.
    #[must_use]
    pub fn find<'t>(tags: &'t [Self], name: &str) -> Option<&'t Self> {
        tags.iter().find(|tag| tag.name == name)
    }

    /// Returns every tag named `name` in `tags`.
    ///
    /// Only `repeated` tags can have multiple matches.
    pub fn find_all<'t>(tags: &'t [Self], name: &'t str) -> impl Iterator<Item = &'t Self> {
        tags.iter().filter(move |tag| tag.name == name)
    }

    /// Returns the text value, or `None` for a nontextual tag.
    #[must_use]
    pub fn text(&self) -> Option<&'static str> {
        match self.value {
            TagValue::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Returns the integer value, or `None` for a noninteger tag.
    #[must_use]
    pub fn integer(&self) -> Option<i128> {
        match self.value {
            TagValue::Integer(integer) => Some(integer),
            _ => None,
        }
    }
}
