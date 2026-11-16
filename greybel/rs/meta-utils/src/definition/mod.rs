mod function;
mod localized_definition;
mod payload;

pub use function::{Argument, ArgumentDefault, Function, Mutation};
pub use localized_definition::LocalizedDefinition;
use serde::Deserialize;

use crate::type_meta::TypeMeta;
use crate::variation::Variation;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "payload::DefinitionPayload")]
pub struct Definition {
    type_meta: TypeMeta,
    is_protected: bool,
    description: Option<String>,
    example: Option<Vec<String>>,
    variations: Vec<Variation>,
    tags: Vec<String>,
    function: Option<Function>,
}

impl Definition {
    pub const DETACHED_TAG: &'static str = "detached";

    #[must_use]
    pub fn type_meta(&self) -> &TypeMeta {
        &self.type_meta
    }

    #[must_use]
    pub fn is_protected(&self) -> bool {
        self.is_protected
    }

    pub(crate) fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub(crate) fn example(&self) -> Option<&[String]> {
        self.example.as_deref()
    }

    #[must_use]
    pub fn variations(&self) -> &[Variation] {
        &self.variations
    }

    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|candidate| candidate == tag)
    }

    /// Whether the definition is a method that is also callable as a general function with the object first, e.g.
    /// `name(file)`.
    #[must_use]
    pub fn is_detached(&self) -> bool {
        self.has_tag(Self::DETACHED_TAG)
    }

    #[must_use]
    pub fn as_function(&self) -> Option<&Function> {
        self.function.as_ref()
    }

    #[must_use]
    pub fn is_function(&self) -> bool {
        self.function.is_some()
    }
}
