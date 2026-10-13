use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::type_meta::TypeMeta;
use crate::variation::Variation;

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub(super) id: String,
    pub(super) arguments: Vec<Argument>,
    pub(super) returns: Vec<TypeMeta>,
    pub(super) return_variations: Vec<Variation>,
    pub(super) mutation: Option<Mutation>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mutation {
    pub(super) fixed_types: IndexMap<String, TypeMeta>,
}

impl Mutation {
    #[must_use]
    pub fn fixed_types(&self) -> &IndexMap<String, TypeMeta> {
        &self.fixed_types
    }
}

impl Function {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn arguments(&self) -> &[Argument] {
        &self.arguments
    }

    #[must_use]
    pub fn argument(&self, label: &str) -> Option<&Argument> {
        self.arguments
            .iter()
            .find(|argument| argument.label == label)
    }

    #[must_use]
    pub fn returns(&self) -> &[TypeMeta] {
        &self.returns
    }

    #[must_use]
    pub fn return_variations(&self) -> &[Variation] {
        &self.return_variations
    }

    #[must_use]
    pub fn mutation(&self) -> Option<&Mutation> {
        self.mutation.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "super::payload::ArgumentPayload")]
pub struct Argument {
    pub(super) label: String,
    pub(super) types: Vec<TypeMeta>,
    #[serde(rename = "opt")]
    pub(super) optional: bool,
    pub(super) default: Option<ArgumentDefault>,
}

impl Argument {
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    #[must_use]
    pub fn types(&self) -> &[TypeMeta] {
        &self.types
    }

    #[must_use]
    pub fn is_optional(&self) -> bool {
        self.optional
    }

    #[must_use]
    pub fn default(&self) -> Option<&ArgumentDefault> {
        self.default.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "lowercase",
    deny_unknown_fields
)]
pub enum ArgumentDefault {
    String(String),
    Number(f64),
}
