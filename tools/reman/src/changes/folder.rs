//! Changes folder (`[repository] changes`): the pending change files in `pending/`, those of each release in a
//! folder named by its date (`2026-11-08/`, `2026-11-08-2/` for a second that day), and the release record.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use indexmap::IndexMap;
use jiff::civil::Date;

use super::{ChangeFile, Level, ReleaseRecord};
use crate::edits::EditTable;
use crate::repository::Repository;

/// Name of a change file whose note gives none.
const DEFAULT_NAME: &str = "change";
/// Number of note words used for its file name.
const NAME_WORDS: usize = 6;
/// Folder of the change files no release consumed yet.
const UNRELEASED_FOLDER: &str = "pending";

pub struct ChangeFolder<'a> {
    repository: &'a Repository,
    /// Root-relative path.
    path: PathBuf,
}

impl<'a> ChangeFolder<'a> {
    pub fn new(repository: &'a Repository, path: PathBuf) -> Self {
        ChangeFolder { repository, path }
    }

    /// Root-relative path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Pending change files' folder, relative to the root.
    pub fn pending_path(&self) -> PathBuf {
        self.path.join(UNRELEASED_FOLDER)
    }

    /// Folder a release on `date` moves the change files it consumes into, relative to the root: named by the
    /// date, numbered past the folders of the releases earlier that day.
    pub fn release_path(&self, date: Date) -> PathBuf {
        let exists = |name: &str| self.repository.root.join(&self.path).join(name).exists();
        let mut name = date.to_string();
        let mut counter = 2;

        while exists(&name) {
            name = format!("{date}-{counter}");
            counter += 1;
        }

        self.path.join(name)
    }

    /// Release record's path, relative to the root.
    pub fn record_path(&self) -> PathBuf {
        self.path.join(ReleaseRecord::FILE_NAME)
    }

    /// Release record, if there was a release.
    pub fn record(&self) -> Result<Option<ReleaseRecord>> {
        ReleaseRecord::read(&self.repository.root.join(self.record_path()))
    }

    /// Pending change files, by file name. A change file directly in the changes folder is an error: it would be
    /// neither pending nor released.
    pub fn files(&self) -> Result<Vec<ChangeFile>> {
        if let Some(loose) = Self::change_files_in(&self.repository.root, &self.path)?.first() {
            bail!(
                "{} is not in {}: pending change files go there",
                loose.display(),
                self.pending_path().display()
            );
        }

        let mut files = Vec::new();

        for path in Self::change_files_in(&self.repository.root, &self.pending_path())? {
            let text = std::fs::read_to_string(self.repository.root.join(&path))
                .with_context(|| format!("reading {}", path.display()))?;

            files.push(ChangeFile::parse(&path, &text)?);
        }

        files.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(files)
    }

    /// New change file naming `entries` (`project:level`), with `note`.
    pub fn create(&self, entries: &[String], note: &str) -> Result<EditTable> {
        let mut levels = IndexMap::new();

        for entry in entries {
            let Some((name, level)) = entry.rsplit_once(':') else {
                bail!("`{entry}` is not `project:level`");
            };
            let node = self.repository.resolve(name)?;

            levels.insert(self.repository.nodes[node].id.clone(), Level::parse(level)?);
        }

        let path = self.pending_path().join(self.file_name(note));
        let text = ChangeFile::render(&levels, note)?;
        let mut edits = EditTable::new(&self.repository.root);

        ChangeFile::parse(&path, &text)?;
        edits.write(&path, text);

        Ok(edits)
    }

    /// File name for a new change: its note's first words as a slug, numbered past those that exist.
    pub fn file_name(&self, note: &str) -> String {
        let slug = slug::slugify(
            note.split_whitespace()
                .take(NAME_WORDS)
                .collect::<Vec<_>>()
                .join(" "),
        );
        let slug = if slug.is_empty() {
            DEFAULT_NAME.to_owned()
        } else {
            slug
        };
        let exists = |name: &str| {
            self.repository
                .root
                .join(self.pending_path())
                .join(name)
                .exists()
        };
        let mut name = format!("{slug}.md");
        let mut counter = 2;

        while exists(&name) {
            name = format!("{slug}-{counter}.md");
            counter += 1;
        }

        name
    }

    /// Markdown files directly in `folder` (relative to `root`), but a README, relative to the root.
    fn change_files_in(root: &Path, folder: &Path) -> Result<Vec<PathBuf>> {
        let Ok(entries) = std::fs::read_dir(root.join(folder)) else {
            return Ok(Vec::new());
        };
        let mut paths = Vec::new();

        for entry in entries {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };

            if entry.file_type()?.is_file()
                && name.ends_with(".md")
                && !name.eq_ignore_ascii_case("README.md")
            {
                paths.push(folder.join(name));
            }
        }

        Ok(paths)
    }
}
