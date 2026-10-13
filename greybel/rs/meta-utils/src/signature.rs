use indexmap::IndexMap;
use serde::Deserialize;

use crate::definition::{Definition, LocalizedDefinition};
use crate::description::DescriptionTable;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignaturePayload {
    #[serde(rename = "type")]
    pub type_name: String,
    pub extends: Option<String>,
    pub hidden: Option<bool>,
    #[serde(rename = "typeParameters")]
    pub type_parameters: Option<Vec<String>>,
    pub definitions: IndexMap<String, Definition>,
}

#[derive(Debug, Clone)]
pub struct Signature {
    type_name: String,
    extends: Option<String>,
    hidden: bool,
    type_parameters: Vec<String>,
    definitions: IndexMap<String, Definition>,
    descriptions: IndexMap<String, DescriptionTable>,
}

impl Signature {
    pub(crate) fn new(type_name: String) -> Self {
        Self {
            type_name,
            extends: None,
            hidden: false,
            type_parameters: Vec::new(),
            definitions: IndexMap::new(),
            descriptions: IndexMap::new(),
        }
    }

    #[must_use]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    #[must_use]
    pub fn extends(&self) -> Option<&str> {
        self.extends.as_deref()
    }

    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    #[must_use]
    pub fn type_parameters(&self) -> &[String] {
        &self.type_parameters
    }

    #[must_use]
    pub fn definitions(&self) -> &IndexMap<String, Definition> {
        &self.definitions
    }

    #[must_use]
    pub fn definition(&self, property: &str, language: &str) -> Option<LocalizedDefinition<'_>> {
        let (name, definition) = self.definitions.get_key_value(property)?;

        Some(self.localize(name, definition, language))
    }

    pub(crate) fn definition_at(
        &self,
        position: usize,
        language: &str,
    ) -> Option<LocalizedDefinition<'_>> {
        let (name, definition) = self.definitions.get_index(position)?;

        Some(self.localize(name, definition, language))
    }

    fn localize<'a>(
        &'a self,
        name: &'a str,
        definition: &'a Definition,
        language: &str,
    ) -> LocalizedDefinition<'a> {
        let description = self
            .descriptions(language)
            .and_then(|descriptions| descriptions.members().get(name));

        LocalizedDefinition::new(&self.type_name, name, definition, description)
    }

    #[must_use]
    pub fn descriptions(&self, language: &str) -> Option<&DescriptionTable> {
        self.descriptions.get(language)
    }

    pub fn languages(&self) -> impl Iterator<Item = &str> {
        self.descriptions.keys().map(String::as_str)
    }

    pub(crate) fn merge(&mut self, payload: SignaturePayload) {
        if let Some(extends) = payload.extends {
            self.extends = Some(extends);
        }

        if let Some(hidden) = payload.hidden {
            self.hidden = hidden;
        }

        if let Some(type_parameters) = payload.type_parameters {
            self.type_parameters = type_parameters;
        }

        self.definitions.extend(payload.definitions);
    }

    pub(crate) fn merge_descriptions(&mut self, language: &str, descriptions: DescriptionTable) {
        self.descriptions
            .entry(language.to_owned())
            .or_default()
            .merge(descriptions);
    }
}
