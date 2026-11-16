use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Values for `#envar`, loaded from `NAME=value` lines in `files` and then `values`; later entries override
/// earlier entries with the same name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct VariableSettings {
    /// Project-relative files containing `NAME=value` lines; missing files contribute no values.
    #[schemars(extend("x-vscode-format" = "commaList"))]
    pub files: Vec<String>,
    pub values: BTreeMap<String, String>,
}

impl VariableSettings {
    pub const DEFAULT_FILE: &str = ".env";
}

impl Default for VariableSettings {
    fn default() -> Self {
        Self {
            files: vec![Self::DEFAULT_FILE.to_owned()],
            values: BTreeMap::new(),
        }
    }
}

impl VariableSettings {
    #[must_use]
    pub fn resolve<'t>(
        &self,
        contents: impl IntoIterator<Item = Option<&'t str>>,
    ) -> BTreeMap<String, String> {
        let mut values = BTreeMap::new();

        for content in contents.into_iter().flatten() {
            VariableFile::new(content).insert_into(&mut values);
        }

        values.extend(self.values.clone());

        values
    }
}

/// Text with one `NAME=value` assignment per line; blank lines and lines starting with `#` are skipped.
#[derive(Debug, Clone, Copy)]
pub struct VariableFile<'t> {
    text: &'t str,
}

impl<'t> VariableFile<'t> {
    #[must_use]
    pub fn new(text: &'t str) -> Self {
        Self { text }
    }

    pub fn assignments(self) -> impl Iterator<Item = (&'t str, &'t str)> {
        self.text.lines().filter_map(|line| {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                return None;
            }

            let (name, value) = line.split_once('=')?;
            let name = name.trim();

            (!name.is_empty() && !value.is_empty()).then_some((name, value))
        })
    }

    pub fn insert_into(self, values: &mut BTreeMap<String, String>) {
        for (name, value) in self.assignments() {
            values.insert(name.to_owned(), value.to_owned());
        }
    }
}
