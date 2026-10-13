//! Change files: one Markdown file per change in the changes folder, naming the projects it changes and how far
//! each moves, then the note their changelogs get.
//!
//! ```text
//! ---
//! rs/greybel-interpreter: minor
//! greybel-vs: patch
//! ---
//!
//! `wait` follows the world clock instead of the wall clock.
//! ```
//!
//! Folder also has the release record (`release.toml`).

mod folder;
mod level;
mod record;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
pub use folder::ChangeFolder;
use gray_matter::engine::YAML;
use gray_matter::{Matter, Pod};
use indexmap::IndexMap;
pub use level::Level;
pub use record::ReleaseRecord;

/// Change file, as written: project names are resolved by the caller.
#[derive(Clone, Debug)]
pub struct ChangeFile {
    /// Root-relative path.
    pub path: PathBuf,
    pub levels: IndexMap<String, Level>,
    pub note: String,
}

impl ChangeFile {
    /// Change file: its YAML front matter maps project names to levels, the remainder is its note.
    pub fn parse(path: &Path, text: &str) -> Result<Self> {
        let entity = Matter::<YAML>::new()
            .parse::<Pod>(text)
            .map_err(|error| anyhow!("{}: {error}", path.display()))?;

        if entity.data.is_none() {
            bail!(
                "{}: a change file starts with its projects between `---` lines",
                path.display()
            );
        }

        let levels: IndexMap<String, Level> = serde_yaml_ng::from_str(&entity.matter)
            .with_context(|| format!("{}: the project list", path.display()))?;
        let note = entity.content.trim().to_owned();

        if levels.is_empty() {
            bail!("{}: names no project", path.display());
        }

        if note.is_empty() && levels.values().any(|level| *level != Level::None) {
            bail!("{}: a change that releases needs a note", path.display());
        }

        Ok(ChangeFile {
            path: path.to_owned(),
            levels,
            note,
        })
    }

    pub fn render(levels: &IndexMap<String, Level>, note: &str) -> Result<String> {
        Ok(format!(
            "---\n{}---\n\n{}\n",
            serde_yaml_ng::to_string(levels)?,
            note.trim()
        ))
    }
}
