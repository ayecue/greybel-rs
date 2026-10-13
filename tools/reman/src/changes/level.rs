use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString, VariantNames};

/// Project version increment; `None` declares no release.
#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Display,
    EnumString,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    VariantNames,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Level {
    None,
    Patch,
    Minor,
    Major,
}

impl Level {
    pub fn parse(text: &str) -> Result<Self> {
        text.parse()
            .map_err(|_| anyhow!("`{text}` is not a level: {}", Level::VARIANTS.join(", ")))
    }

    pub fn heading(self) -> &'static str {
        match self {
            Level::Major => "Major changes",
            Level::Minor => "Minor changes",
            Level::Patch | Level::None => "Patch changes",
        }
    }

    /// Version increment from `previous` to `current`, both dotted numbers (`1.2`, `1.2.3`): `Major` for a different first
    /// differs, `Minor` when the second does, else `Patch`.
    pub fn between(previous: &str, current: &str) -> Option<Level> {
        let parse = |text: &str| {
            text.split('.')
                .map(str::parse::<u64>)
                .collect::<Result<Vec<_>, _>>()
                .ok()
        };
        let (previous, current) = (parse(previous)?, parse(current)?);

        Some(if previous.first() != current.first() {
            Level::Major
        } else if previous.get(1) != current.get(1) {
            Level::Minor
        } else {
            Level::Patch
        })
    }

    /// Applies this level to `version` using semver arithmetic, including 0.x versions.
    pub fn bump(self, version: &semver::Version) -> semver::Version {
        match self {
            Level::None => version.clone(),
            Level::Patch => semver::Version::new(version.major, version.minor, version.patch + 1),
            Level::Minor => semver::Version::new(version.major, version.minor + 1, 0),
            Level::Major => semver::Version::new(version.major + 1, 0, 0),
        }
    }
}
