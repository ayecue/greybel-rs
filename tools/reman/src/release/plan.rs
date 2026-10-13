//! Release worked out in one pass over every pending change file: each project moves once, by the furthest any
//! change moves it, however many changes and dependencies reach it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::changes::{ChangeFile, Level, ReleaseRecord};
use crate::language::{Scope, VersionSource};
use crate::repository::Repository;

/// Change file with its projects resolved.
#[derive(Clone, Debug)]
pub struct Change {
    pub path: PathBuf,
    pub levels: Vec<(usize, Level)>,
    pub note: String,
}

#[derive(Debug)]
pub struct Plan {
    /// Sorted by dependencies.
    pub releases: Vec<Release>,
    pub change_files: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct Release {
    pub node: usize,
    pub previous: String,
    pub version: String,
    pub level: Level,
    pub notes: Vec<Note>,
}

#[derive(Clone, Debug)]
pub struct Note {
    pub level: Level,
    /// Projects the change was made in, when it reaches this one through them.
    pub through: Vec<usize>,
    pub text: String,
    pub change: PathBuf,
}

impl Change {
    /// Resolves project names in `file` and requires every name to identify a project.
    pub fn resolve(repository: &Repository, file: &ChangeFile) -> Result<Change> {
        let mut levels = Vec::new();

        for (name, level) in &file.levels {
            let node = repository
                .resolve(name)
                .with_context(|| file.path.display().to_string())?;

            if !repository.nodes[node].is_project() {
                bail!(
                    "{}: {} has no [project] (dev and test packages are never released)",
                    file.path.display(),
                    repository.nodes[node].id
                );
            }

            levels.push((node, *level));
        }

        Ok(Change {
            path: file.path.clone(),
            levels,
            note: file.note.clone(),
        })
    }

    /// Note it gives each project it reaches: its own level to those it names, a patch (through the projects
    /// it names) to those depending on them.
    fn notes(&self, repository: &Repository) -> BTreeMap<usize, Note> {
        let levels = self.levels.iter().copied().collect::<BTreeMap<_, _>>();
        let mut reach: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();

        for (&node, _) in levels.iter().filter(|(_, level)| **level != Level::None) {
            for dependent in repository.with_dependents([node], &[Scope::Normal]) {
                reach.entry(dependent).or_default().insert(node);
            }
        }

        reach
            .into_iter()
            .filter(|(node, _)| repository.nodes[*node].is_project())
            .map(|(node, through)| {
                let note = match levels
                    .get(&node)
                    .copied()
                    .filter(|level| *level > Level::None)
                {
                    Some(level) => self.note(level, Vec::new()),
                    None => self.note(
                        Level::Patch,
                        through.into_iter().filter(|other| *other != node).collect(),
                    ),
                };

                (node, note)
            })
            .collect()
    }

    fn note(&self, level: Level, through: Vec<usize>) -> Note {
        Note {
            level,
            through,
            text: self.note.clone(),
            change: self.path.clone(),
        }
    }
}

impl Plan {
    /// Plan of the repository's pending change files.
    pub fn from_change_files(repository: &Repository) -> Result<Plan> {
        let folder = repository.changes();
        let changes = folder
            .files()?
            .iter()
            .map(|file| Change::resolve(repository, file))
            .collect::<Result<Vec<_>>>()?;

        Plan::new(repository, &changes, folder.record()?.as_ref())
    }

    /// Projects reached by `changes`, in dependency order, with the largest note increment; projects whose
    /// versions its file names declare is at the version it declares (its previous one the `record`'s).
    pub fn new(
        repository: &Repository,
        changes: &[Change],
        record: Option<&ReleaseRecord>,
    ) -> Result<Plan> {
        let mut notes: BTreeMap<usize, Vec<Note>> = BTreeMap::new();

        for change in changes {
            for (node, note) in change.notes(repository) {
                notes.entry(node).or_default().push(note);
            }
        }

        let mut releases = Vec::new();

        for node in repository.node_order() {
            if let Some(notes) = notes.remove(&node) {
                releases.push(Release::new(repository, node, notes, record)?);
            }
        }

        Ok(Plan {
            releases,
            change_files: changes.iter().map(|change| change.path.clone()).collect(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.releases.is_empty() && self.change_files.is_empty()
    }

    /// Plan's version of the project `node`, if it releases it.
    pub fn version_of(&self, node: usize) -> Option<&str> {
        self.releases
            .iter()
            .find(|release| release.node == node)
            .map(|release| release.version.as_str())
    }

    /// Human-readable release plan.
    pub fn describe(&self, repository: &Repository) -> String {
        let mut text = String::new();

        if self.releases.is_empty() {
            text.push_str("Nothing to release.\n");
        }

        for release in &self.releases {
            let node = &repository.nodes[release.node];
            let reasons = release
                .notes
                .iter()
                .map(|note| note.reason(repository))
                .collect::<Vec<_>>();

            let _ = writeln!(
                text,
                "{} {} -> {} ({}{})\n    {}",
                node.id,
                release.previous,
                release.version,
                release.level,
                if node.is_released() { ", tagged" } else { "" },
                reasons.join("\n    ")
            );
        }

        if !self.change_files.is_empty() {
            let _ = writeln!(text, "Consumes {} change file(s).", self.change_files.len());
        }

        text
    }
}

impl Release {
    fn new(
        repository: &Repository,
        node: usize,
        notes: Vec<Note>,
        record: Option<&ReleaseRecord>,
    ) -> Result<Self> {
        let package = &repository.nodes[node];
        let level = notes
            .iter()
            .map(|note| note.level)
            .max()
            .unwrap_or(Level::None);
        let current = package
            .package
            .version
            .clone()
            .with_context(|| format!("{} has no version", package.id))?;
        let (previous, version) = match package.language.version_source() {
            VersionSource::Manifest => {
                let semver_version = semver::Version::parse(&current).with_context(|| {
                    format!("{}: `{current}` is not a semver version", package.id)
                })?;

                (current.clone(), level.bump(&semver_version).to_string())
            }
            VersionSource::FileNames => {
                let previous = record
                    .and_then(|record| record.versions.get(&package.id))
                    .cloned();

                (previous.unwrap_or_else(|| current.clone()), current)
            }
        };

        Ok(Release {
            node,
            previous,
            version,
            level,
            notes,
        })
    }
}

impl Note {
    /// Version increment rationale: change file, level, and intermediary projects.
    fn reason(&self, repository: &Repository) -> String {
        match self.through.as_slice() {
            [] => format!("{} ({})", self.change.display(), self.level),
            through => {
                let names = through
                    .iter()
                    .map(|node| repository.display_name(*node))
                    .collect::<Vec<_>>();

                format!("{} through {}", self.change.display(), names.join(", "))
            }
        }
    }
}
