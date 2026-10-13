use std::collections::BTreeMap;

use greybel_documents::Uri;

use crate::manifest::{Manifest, VariableSettings};
use crate::profile::ProfileError;

/// Project settings from `greybel.json` and their base folder, or defaults when no manifest is found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Project {
    /// Folder containing the project's `greybel.json`, when present.
    pub root: Option<Uri>,
    pub manifest: Manifest,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("{uri}: {error}")]
    Invalid {
        uri: String,
        #[source]
        error: serde_json::Error,
    },
    #[error("{uri}: {error}")]
    Profile {
        uri: String,
        #[source]
        error: ProfileError,
    },
}

impl Project {
    pub const FILE_NAME: &str = "greybel.json";

    /// Candidate `greybel.json` locations for document `uri`, from its folder upward in nearest-first order.
    #[must_use]
    pub fn locations(uri: &Uri) -> Vec<Uri> {
        let mut locations = Vec::new();
        let mut folder = uri.dirname().ok();

        while let Some(current) = folder {
            if let Ok(location) = current.join_path(&[Self::FILE_NAME]) {
                locations.push(location);
            }

            folder = current.dirname().ok().filter(|parent| *parent != current);
        }

        locations
    }

    /// Reads `text` as the `greybel.json` at `uri` and validates every profile.
    pub fn read(uri: &Uri, text: &str) -> Result<Self, ProjectError> {
        let manifest: Manifest =
            serde_json::from_str(text).map_err(|error| ProjectError::Invalid {
                uri: uri.to_string(),
                error,
            })?;

        manifest
            .validate_profiles()
            .map_err(|error| ProjectError::Profile {
                uri: uri.to_string(),
                error,
            })?;

        Ok(Self {
            root: uri.dirname().ok(),
            manifest,
        })
    }

    /// Discovers the project for document `uri` using the first [`Self::locations`] path for which `read` returns a
    /// file; uses defaults if none exists.
    pub fn discover(
        uri: &Uri,
        mut read: impl FnMut(&Uri) -> Option<String>,
    ) -> Result<Self, ProjectError> {
        for location in Self::locations(uri) {
            if let Some(text) = read(&location) {
                return Self::read(&location, &text);
            }
        }

        Ok(Self::default())
    }

    /// Returns this project with profile `name` applied, or unchanged when no name is supplied.
    pub fn with_profile(self, name: Option<&str>) -> Result<Self, ProjectError> {
        let Some(name) = name else {
            return Ok(self);
        };
        let manifest = self
            .manifest
            .with_profile(name)
            .map_err(|error| ProjectError::Profile {
                uri: self.location(),
                error,
            })?;

        Ok(Self { manifest, ..self })
    }

    /// Returns the project's `greybel.json` location or indicates that none exists.
    fn location(&self) -> String {
        self.root
            .as_ref()
            .and_then(|root| root.join_path(&[Self::FILE_NAME]).ok())
            .map_or_else(
                || "the defaults (no greybel.json)".to_owned(),
                |uri| uri.to_string(),
            )
    }

    /// Resolves `path` to a URI relative to the project folder, or `folder` when no `greybel.json` exists.
    #[must_use]
    pub fn resolve(&self, folder: &Uri, path: &str) -> Option<Uri> {
        self.root
            .as_ref()
            .unwrap_or(folder)
            .resolve_path(&[path])
            .ok()
    }

    /// Resolves project entries relative to the project folder, or `folder` without `greybel.json`.
    #[must_use]
    pub fn entries(&self, folder: &Uri) -> Vec<Uri> {
        self.manifest
            .entries
            .iter()
            .filter_map(|entry| self.resolve(folder, entry))
            .collect()
    }

    /// Resolves `variables` files relative to the project folder, or `folder` without `greybel.json`.
    #[must_use]
    pub fn variable_files(&self, folder: &Uri, variables: &VariableSettings) -> Vec<Uri> {
        variables
            .files
            .iter()
            .filter_map(|file| self.resolve(folder, file))
            .collect()
    }

    /// Resolves `variables`, reading its files through `read`; `None` denotes a missing file.
    #[must_use]
    pub fn read_variables(
        &self,
        folder: &Uri,
        variables: &VariableSettings,
        read: impl FnMut(&Uri) -> Option<String>,
    ) -> BTreeMap<String, String> {
        let contents: Vec<Option<String>> = self
            .variable_files(folder, variables)
            .iter()
            .map(read)
            .collect();

        variables.resolve(contents.iter().map(Option::as_deref))
    }
}

impl VariableSettings {
    /// Resolves values from `contents` in `files` order, treating `None` as missing, followed by explicit `values`.
    #[must_use]
    pub fn resolve<'t>(
        &self,
        contents: impl IntoIterator<Item = Option<&'t str>>,
    ) -> BTreeMap<String, String> {
        let mut values = BTreeMap::new();

        for content in contents.into_iter().flatten() {
            insert_assignments(content.lines(), &mut values);
        }

        values.extend(self.values.clone());

        values
    }
}

/// Adds `NAME=value` assignments from `lines` to `values`, ignoring blank lines, `#` comments, and empty names
/// or values.
pub fn insert_assignments<'l>(
    lines: impl IntoIterator<Item = &'l str>,
    values: &mut BTreeMap<String, String>,
) {
    for line in lines {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();

        if !name.is_empty() && !value.is_empty() {
            values.insert(name.to_owned(), value.to_owned());
        }
    }
}
