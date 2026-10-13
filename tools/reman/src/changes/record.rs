//! Stores each project's last-released version in `release.toml` under the changes folder.
//! The next release accounts for changes starting at the last commit that wrote this record.

use std::path::Path;

use anyhow::{Context, Result};
use indexmap::IndexMap;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseRecord {
    pub date: Date,
    pub versions: IndexMap<String, String>,
}

impl ReleaseRecord {
    pub const FILE_NAME: &str = "release.toml";

    /// Record at `path`, if there is one.
    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let text = std::fs::read_to_string(path)?;

        toml::from_str(&text)
            .map(Some)
            .with_context(|| format!("parsing {}", path.display()))
    }

    pub fn render(&self) -> Result<String> {
        let header = "# Every project's version at the last release (`reman release prepare`). The last commit writing this\n\
                      # file is where the changes the next release accounts for start.\n\n";

        Ok(format!("{header}{}", toml::to_string(self)?))
    }
}
