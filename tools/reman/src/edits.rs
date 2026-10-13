//! Buffers file edits before writing so dry-run output matches the real write set exactly.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use indexmap::IndexMap;
use similar::TextDiff;

use crate::paths::PathExt;

#[derive(Debug)]
enum Edit {
    Write(String),
    Delete,
}

/// Repository-root-relative paths for pending writes and deletions.
#[derive(Debug)]
pub struct EditTable {
    root: PathBuf,
    edits: IndexMap<PathBuf, Edit>,
}

impl EditTable {
    pub fn new(root: &Path) -> Self {
        EditTable {
            root: root.to_owned(),
            edits: IndexMap::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// File with the pending edits applied (None if missing or deleted).
    pub fn read(&self, path: &Path) -> Result<Option<String>> {
        match self.edits.get(path) {
            Some(Edit::Write(text)) => Ok(Some(text.clone())),
            Some(Edit::Delete) => Ok(None),
            None => {
                let absolute = self.root.join(path);

                if !absolute.exists() {
                    return Ok(None);
                }

                std::fs::read_to_string(&absolute)
                    .map(Some)
                    .with_context(|| format!("reading {}", path.display()))
            }
        }
    }

    pub fn write(&mut self, path: &Path, text: String) {
        self.edits.insert(path.to_owned(), Edit::Write(text));
    }

    pub fn delete(&mut self, path: &Path) {
        self.edits.insert(path.to_owned(), Edit::Delete);
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.edits.keys().map(PathBuf::as_path)
    }

    /// Pending writes.
    pub fn writes(&self) -> Vec<(PathBuf, String)> {
        self.edits
            .iter()
            .filter_map(|(path, edit)| match edit {
                Edit::Write(text) => Some((path.clone(), text.clone())),
                Edit::Delete => None,
            })
            .collect()
    }

    /// Patch diff of every pending change.
    pub fn diff(&self) -> Result<String> {
        let mut output = String::new();

        for (path, edit) in &self.edits {
            let absolute = self.root.join(path);
            let before = if absolute.exists() {
                std::fs::read_to_string(&absolute)?
            } else {
                String::new()
            };
            let after = match edit {
                Edit::Write(text) => text.as_str(),
                Edit::Delete => "",
            };
            let name = path.to_slash();
            let diff = TextDiff::from_lines(before.as_str(), after);
            let patch = diff
                .unified_diff()
                .context_radius(2)
                .header(&format!("a/{name}"), &format!("b/{name}"))
                .to_string();

            if !patch.is_empty() {
                writeln!(output, "{patch}")?;
            }
        }

        Ok(output)
    }

    /// Applies edits, or prints their diff and reports the action when `dry_run` is set.
    pub fn save(self, dry_run: bool) -> Result<()> {
        if self.is_empty() {
            println!("nothing to write");
        } else if dry_run {
            print!("{}", self.diff()?);
            println!("(dry run: nothing written)");
        } else {
            let paths = self
                .paths()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>();

            self.apply()?;
            println!("wrote {}", paths.join(", "));
        }

        Ok(())
    }

    pub fn apply(self) -> Result<()> {
        for (path, edit) in self.edits {
            let absolute = self.root.join(&path);

            match edit {
                Edit::Write(text) => {
                    if let Some(parent) = absolute.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    std::fs::write(&absolute, text)
                        .with_context(|| format!("writing {}", path.display()))?;
                }
                Edit::Delete => {
                    if absolute.exists() {
                        std::fs::remove_file(&absolute)
                            .with_context(|| format!("deleting {}", path.display()))?;
                    }
                }
            }
        }

        Ok(())
    }
}
