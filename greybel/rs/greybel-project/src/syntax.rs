use std::borrow::Cow;
use std::collections::BTreeMap;

use greybel_core::SyntaxFeatures;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

/// Syntax features turned on or off by name, e.g. `{"ternary": false, "classes": false}`. A feature not listed is
/// on; strict mode turns all of them off.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SyntaxSettings(pub BTreeMap<String, bool>);

impl SyntaxSettings {
    /// The names of the features turned off.
    pub fn off(&self) -> impl Iterator<Item = &str> {
        self.0
            .iter()
            .filter(|&(_, &on)| !on)
            .map(|(name, _)| name.as_str())
    }

    /// The names of the features turned off, for the builder and runtime protocols; `None` when every feature is on.
    #[must_use]
    pub fn off_names(&self) -> Option<Vec<String>> {
        let names: Vec<String> = self.off().map(str::to_owned).collect();

        (!names.is_empty()).then_some(names)
    }

    /// The features to accept; names the parser doesn't know are skipped.
    #[must_use]
    pub fn features(&self) -> SyntaxFeatures {
        SyntaxFeatures::except(self.off())
    }
}

impl JsonSchema for SyntaxSettings {
    fn schema_name() -> Cow<'static, str> {
        "SyntaxSettings".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        let names: Vec<&str> = SyntaxFeatures::NAMES
            .iter()
            .map(|&(name, _)| name)
            .collect();

        json_schema!({
            "type": "object",
            "propertyNames": { "enum": names },
            "additionalProperties": { "type": "boolean" },
        })
    }
}
