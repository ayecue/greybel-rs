#[cfg(feature = "builder")]
mod builder;
#[cfg(feature = "runtime")]
mod runtime;

use std::collections::BTreeMap;

use greybel_documents::Uri;
#[cfg(feature = "runtime")]
pub use runtime::Place;

use crate::manifest::{Manifest, ProfileError, VariableSettings};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Project {
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

    fn location(&self) -> String {
        self.root
            .as_ref()
            .and_then(|root| root.join_path(&[Self::FILE_NAME]).ok())
            .map_or_else(
                || "the defaults (no greybel.json)".to_owned(),
                |uri| uri.to_string(),
            )
    }

    #[must_use]
    pub fn resolve(&self, folder: &Uri, path: &str) -> Option<Uri> {
        self.root
            .as_ref()
            .unwrap_or(folder)
            .resolve_path(&[path])
            .ok()
    }

    #[must_use]
    pub fn entries(&self, folder: &Uri) -> Vec<Uri> {
        self.manifest
            .entries
            .iter()
            .filter_map(|entry| self.resolve(folder, entry))
            .collect()
    }

    #[must_use]
    pub fn variable_files(&self, folder: &Uri, variables: &VariableSettings) -> Vec<Uri> {
        variables
            .files
            .iter()
            .filter_map(|file| self.resolve(folder, file))
            .collect()
    }

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
