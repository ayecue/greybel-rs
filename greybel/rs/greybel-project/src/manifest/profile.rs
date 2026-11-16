use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::manifest::{BuildSettings, Manifest, RunSettings};

/// Named overrides for project `build` and `run` settings. Objects merge by key; all other values, including lists,
/// replace the original value entirely.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Profile {
    /// Parent profile to modify instead of the base settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    /// Build-setting overrides for this profile.
    #[serde(skip_serializing_if = "Map::is_empty")]
    #[schemars(with = "BuildSettings")]
    pub build: Map<String, Value>,
    /// Run-setting overrides for this profile.
    #[serde(skip_serializing_if = "Map::is_empty")]
    #[schemars(with = "RunSettings")]
    pub run: Map<String, Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("the project has no profile \"{name}\"")]
    Unknown { name: String },
    #[error("the profiles extend each other in a cycle: {}", names.join(" -> "))]
    Cycle { names: Vec<String> },
    #[error("invalid profile \"{name}\": {error}")]
    Invalid {
        name: String,
        #[source]
        error: serde_json::Error,
    },
}

impl Manifest {
    pub fn with_profile(&self, name: &str) -> Result<Self, ProfileError> {
        let chain = self.profile_chain(name)?;
        let mut manifest = self.clone();

        let overlay = ProfileOverlay { name };

        manifest.build = overlay.apply(&self.build, chain.iter().map(|profile| &profile.build))?;
        manifest.run = overlay.apply(&self.run, chain.iter().map(|profile| &profile.run))?;

        Ok(manifest)
    }

    pub(crate) fn validate_profiles(&self) -> Result<(), ProfileError> {
        self.profiles
            .keys()
            .try_for_each(|name| self.with_profile(name).map(drop))
    }

    fn profile_chain(&self, name: &str) -> Result<Vec<&Profile>, ProfileError> {
        let mut names = Vec::new();
        let mut next = Some(name);

        while let Some(current) = next {
            if names.contains(&current) {
                names.push(current);

                return Err(ProfileError::Cycle {
                    names: names.into_iter().map(str::to_owned).collect(),
                });
            }

            let profile = self
                .profiles
                .get(current)
                .ok_or_else(|| ProfileError::Unknown {
                    name: current.to_owned(),
                })?;

            names.push(current);
            next = profile.extends.as_deref();
        }

        Ok(names
            .iter()
            .rev()
            .map(|name| &self.profiles[*name])
            .collect())
    }
}

/// Applies a profile chain's partial settings over the manifest's settings.
struct ProfileOverlay<'n> {
    name: &'n str,
}

impl ProfileOverlay<'_> {
    fn apply<'c, T: Serialize + DeserializeOwned>(
        &self,
        settings: &T,
        changes: impl IntoIterator<Item = &'c Map<String, Value>>,
    ) -> Result<T, ProfileError> {
        let invalid = |error| ProfileError::Invalid {
            name: self.name.to_owned(),
            error,
        };
        let mut value = serde_json::to_value(settings).map_err(invalid)?;

        for change in changes {
            Self::merge(&mut value, change);
        }

        serde_json::from_value(value).map_err(invalid)
    }

    fn merge(value: &mut Value, change: &Map<String, Value>) {
        let Value::Object(object) = value else {
            *value = Value::Object(change.clone());

            return;
        };

        for (key, replacement) in change {
            match (object.get_mut(key), replacement) {
                (Some(current @ Value::Object(_)), Value::Object(inner)) => {
                    Self::merge(current, inner);
                }
                _ => {
                    object.insert(key.clone(), replacement.clone());
                }
            }
        }
    }
}
