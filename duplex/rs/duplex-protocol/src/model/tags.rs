use super::{DocumentationRef, Literal, Span, TypeRef};

/// Generator metadata declaration. Tags do not affect binary layout, fingerprints, or migrations.
#[derive(Debug, Clone, PartialEq)]
pub struct TagDeclaration {
    pub docs: DocumentationRef,
    pub name: String,
    /// Value type, or `None` for a flag.
    pub ty: Option<TypeRef>,
    /// Definition elements that accept this tag.
    pub places: Vec<TagPlace>,
    /// Whether one item can repeat this tag.
    pub repeatable: bool,
    pub(crate) span: Span,
}

/// Definition element that accepts a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagPlace {
    Protocol,
    Message,
    /// Message, struct, or variant field.
    Field,
    Struct,
    Union,
    Variant,
    Enumeration,
    Member,
}

impl TagPlace {
    pub const ALL: [Self; 8] = [
        Self::Protocol,
        Self::Message,
        Self::Field,
        Self::Struct,
        Self::Union,
        Self::Variant,
        Self::Enumeration,
        Self::Member,
    ];

    #[must_use]
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Protocol => "protocol",
            Self::Message => "message",
            Self::Field => "field",
            Self::Struct => "struct",
            Self::Union => "union",
            Self::Variant => "variant",
            Self::Enumeration => "enum",
            Self::Member => "member",
        }
    }

    #[must_use]
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|place| place.keyword() == keyword)
    }
}

/// Item tag annotation: `@name` or `@name(value)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub name: String,
    pub value: Option<Literal>,
    pub(crate) span: Span,
}

/// Declaration-ordered tag annotations.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TagList(Vec<Tag>);

impl TagList {
    pub(crate) fn push(&mut self, tag: Tag) {
        self.0.push(tag);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn all(&self) -> &[Tag] {
        &self.0
    }

    /// Returns tags named `name`. Only `repeated` declarations permit multiple matches.
    pub fn find_all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Tag> {
        self.0.iter().filter(move |tag| tag.name == name)
    }

    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.find_all(name).next().is_some()
    }
}
