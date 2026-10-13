use super::Tag;

/// Message, struct, or union-variant field metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldTagContainer {
    /// Declared field name.
    pub name: &'static str,
    /// Declared field tags.
    pub tags: &'static [Tag],
}

impl FieldTagContainer {
    /// Returns the first field in `fields` with tag `tag`.
    #[must_use]
    pub fn with_tag<'f>(fields: &'f [Self], tag: &str) -> Option<&'f Self> {
        fields
            .iter()
            .find(|field| Tag::find(field.tags, tag).is_some())
    }
}
