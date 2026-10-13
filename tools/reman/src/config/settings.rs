//! Resolves settings through repository project defaults, workspace defaults, then project settings,
//! retaining the originating file for each value.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use super::{Kind, ProjectDefaultTable, ProjectTable};

/// Value and the file that set it.
#[derive(Clone, Debug)]
pub struct Setting<T> {
    pub value: T,
    pub origin: PathBuf,
}

/// Project's settings after inheritance.
#[derive(Clone, Debug)]
pub struct ProjectSettings {
    pub kind: Kind,
    pub release: bool,
    pub latest: bool,
    pub tag: Option<String>,
    pub uses: Vec<String>,
    pub changelog: Setting<String>,
    pub skip: Setting<Vec<String>>,
    pub origin: PathBuf,
}

/// Inherited project defaults for any workspace folder, including non-project folders.
#[derive(Clone, Debug)]
pub struct ProjectBase {
    pub changelog: Setting<String>,
    pub skip: Setting<Vec<String>>,
}

impl ProjectBase {
    const CHANGELOG: &str = "CHANGELOG.md";

    pub fn root() -> Self {
        ProjectBase {
            changelog: Setting {
                value: Self::CHANGELOG.to_owned(),
                origin: PathBuf::from("(reman)"),
            },
            skip: Setting {
                value: Vec::new(),
                origin: PathBuf::from("(reman)"),
            },
        }
    }

    /// Overlays `defaults` from `origin` onto `self`.
    pub fn over(&self, defaults: Option<&ProjectDefaultTable>, origin: &Path) -> Self {
        let Some(defaults) = defaults else {
            return self.clone();
        };

        ProjectBase {
            changelog: match &defaults.changelog {
                Some(value) => Setting {
                    value: value.clone(),
                    origin: origin.to_owned(),
                },
                None => self.changelog.clone(),
            },
            skip: match &defaults.skip {
                Some(value) => Setting {
                    value: value.clone(),
                    origin: origin.to_owned(),
                },
                None => self.skip.clone(),
            },
        }
    }

    pub fn project(&self, project: &ProjectTable, origin: &Path) -> Result<ProjectSettings> {
        let defaults = self.over(
            Some(&ProjectDefaultTable {
                changelog: project.changelog.clone(),
                skip: project.skip.clone(),
            }),
            origin,
        );
        let release = match (project.kind, project.release) {
            (Kind::App, release) => release.unwrap_or(true),
            (_, Some(true)) => bail!(
                "{}: only apps are released (`release = true` on a {})",
                origin.display(),
                project.kind
            ),
            (_, _) => false,
        };

        if project.latest && !release {
            bail!("{}: `latest` needs a released app", origin.display());
        }

        if project.tag.is_some() && !release {
            bail!("{}: `tag` needs a released app", origin.display());
        }

        Ok(ProjectSettings {
            kind: project.kind,
            release,
            latest: project.latest,
            tag: project.tag.clone(),
            uses: project.uses.clone(),
            changelog: defaults.changelog,
            skip: defaults.skip,
            origin: origin.to_owned(),
        })
    }
}
