use super::{
    Constant, Protocol, Span, Status, Tag, TagDeclaration, TypeDeclaration, TypeKind, Value,
    Version,
};
use crate::constraints::ConstraintTable;
use crate::error::DefinitionError;
use crate::fingerprint::Fingerprint;
use crate::syntax::Parser;
use crate::versions::AdditionTable;

/// Parsed, validated duplex definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    pub(crate) core: Option<CoreUse>,
    pub(crate) status: Status,
    pub(crate) protocol: Option<Protocol>,
    pub(crate) types: Vec<TypeDeclaration>,
    pub(crate) constants: Vec<Constant>,
    pub(crate) tags: Vec<TagDeclaration>,
}

/// Declared duplex-core dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreUse {
    /// Required core version.
    pub version: Version,
    pub(crate) span: Span,
}

impl Definition {
    /// Parses and validates definition source.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid syntax or semantics.
    pub fn parse(source: &str) -> Result<Self, DefinitionError> {
        let definition = Parser::new(source)?.parse()?;

        ConstraintTable::of(&definition).verify()?;

        Ok(definition)
    }

    #[must_use]
    pub fn core(&self) -> Option<Version> {
        self.core.map(|core| core.version)
    }

    #[must_use]
    pub fn status(&self) -> &Status {
        &self.status
    }

    #[must_use]
    pub fn protocol(&self) -> Option<&Protocol> {
        self.protocol.as_ref()
    }

    #[must_use]
    pub fn types(&self) -> &[TypeDeclaration] {
        &self.types
    }

    #[must_use]
    pub fn declaration(&self, name: &str) -> Option<&TypeDeclaration> {
        self.types
            .iter()
            .find(|declaration| declaration.name == name)
    }

    /// Returns declared tags.
    #[must_use]
    pub fn tags(&self) -> &[TagDeclaration] {
        &self.tags
    }

    #[must_use]
    pub fn tag_declaration(&self, name: &str) -> Option<&TagDeclaration> {
        self.tags
            .iter()
            .find(|declaration| declaration.name == name)
    }

    /// Resolves `tag` to its declared typed value, or returns `None` for flags and unresolved tags.
    #[must_use]
    pub fn tag_value(&self, tag: &Tag) -> Option<Value> {
        let ty = self.tag_declaration(&tag.name)?.ty.as_ref()?;

        self.value_of(tag.value.as_ref()?, ty).ok()
    }

    #[must_use]
    pub fn is_struct(&self, name: &str) -> bool {
        self.declaration(name)
            .is_some_and(|declaration| matches!(declaration.kind, TypeKind::Struct(_)))
    }

    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        Fingerprint::of(self, &AdditionTable::default())
    }

    #[must_use]
    pub fn fingerprint_of(&self, name: &str) -> Option<u32> {
        self.declaration(name).map(|declaration| {
            Fingerprint::of_type(self, &AdditionTable::default(), &declaration.name)
        })
    }
}
