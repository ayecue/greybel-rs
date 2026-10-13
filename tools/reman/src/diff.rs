//! Packages changed since a commit: assign each file to its innermost containing package, otherwise to
//! the packages reading a folder containing it; a lock file to the packages whose resolved dependencies it changed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config;
use crate::git::Git;
use crate::language::{FileTree, Language, LockChange, Scope};
use crate::paths::{GlobMatcher, PathExt};
use crate::repository::Repository;

#[derive(Debug, Default)]
pub struct Diff {
    /// Commit they are counted from.
    pub base: String,
    /// Release-relevant changes grouped by package.
    pub release_files: BTreeMap<usize, Vec<PathBuf>>,
    /// All changes, including tests, by package.
    pub files: BTreeMap<usize, Vec<PathBuf>>,
    /// Modified files not owned by a package.
    pub orphans: Vec<PathBuf>,
}

impl Diff {
    /// Commit the last release wrote the release record in, else HEAD (nothing released yet: the working
    /// tree's changes only).
    pub fn release_base(repository: &Repository) -> Result<String> {
        let git = Git::open(&repository.root)?;

        match git.last_commit_of(&repository.changes().record_path())? {
            Some(commit) => Ok(commit),
            None => git.head(),
        }
    }

    /// Changes since `revision` (none or empty: the last release), by package.
    pub fn since(repository: &Repository, revision: Option<&str>) -> Result<Diff> {
        let git = Git::open(&repository.root)?;
        let base = match revision {
            Some(revision) if !revision.is_empty() => git.resolve(revision)?,
            _ => Self::release_base(repository)?,
        };
        let changes_folder = repository.changes().path().to_path_buf();
        let locks = repository
            .workspaces
            .iter()
            .flat_map(|workspace| {
                workspace
                    .language
                    .lock_files(&workspace.path)
                    .into_iter()
                    .map(|lock| (lock, workspace.language))
            })
            .collect::<BTreeMap<_, _>>();
        let skips = repository
            .nodes
            .iter()
            .map(|node| GlobMatcher::new(node.skip()))
            .collect::<Result<Vec<_>>>()?;
        let mut diff = Diff {
            base: base.clone(),
            ..Diff::default()
        };

        for file in git.changed_files(&base)? {
            // Excludes change files, reman files, and build output from package ownership, even when tracked.
            if file.starts_with(&changes_folder)
                || file
                    .file_name()
                    .is_some_and(|name| name == config::FILE_NAME)
                || FileTree::is_build_output(&file)
            {
                continue;
            }

            if let Some(&language) = locks.get(&file) {
                diff.add_lock(repository, &git, &file, language)?;
            } else if let Some(node) = repository.owner(&file) {
                let inside = file
                    .strip_prefix(repository.nodes[node].path())
                    .unwrap_or(&file)
                    .to_slash();

                if file != repository.nodes[node].changelog() {
                    diff.add(node, &file, !skips[node].matches(&inside));
                }
            } else {
                diff.add_input(repository, file);
            }
        }

        for files in diff
            .release_files
            .values_mut()
            .chain(diff.files.values_mut())
        {
            files.sort();
            files.dedup();
        }

        Ok(diff)
    }

    /// Packages changed and every package depending on one (tests included): what a task run with `--since`
    /// covers.
    pub fn impact(&self, repository: &Repository) -> BTreeSet<usize> {
        repository.with_dependents(self.files.keys().copied(), &[Scope::Normal, Scope::Dev])
    }

    /// Associates `file` with changed `node`; `released` marks the change as release-relevant.
    fn add(&mut self, node: usize, file: &Path, released: bool) {
        self.files.entry(node).or_default().push(file.to_owned());

        if released {
            self.release_files
                .entry(node)
                .or_default()
                .push(file.to_owned());
        }
    }

    /// File outside every package changes the packages reading a folder containing it.
    fn add_input(&mut self, repository: &Repository, file: PathBuf) {
        let readers = repository
            .inputs
            .iter()
            .filter(|input| file.starts_with(&input.path))
            .collect::<Vec<_>>();

        for input in &readers {
            self.add(input.node, &file, input.scope == Scope::Normal);
        }

        if readers.is_empty() {
            self.orphans.push(file);
        }
    }

    /// Lock file changes the packages whose resolved dependencies differ from those at the base.
    fn add_lock(
        &mut self,
        repository: &Repository,
        git: &Git,
        lock: &Path,
        language: &'static dyn Language,
    ) -> Result<()> {
        let before = git.show(&self.base, lock)?;
        let after = std::fs::read_to_string(repository.root.join(lock)).ok();
        let (Some(before), Some(after)) = (before, after) else {
            return Ok(());
        };
        let mut nodes = BTreeSet::new();

        for change in language.lock_changes(lock, &before, &after)? {
            let node = match change {
                LockChange::Folder(folder) => repository
                    .nodes
                    .iter()
                    .position(|node| node.path() == folder),
                LockChange::Package(name) => repository
                    .nodes
                    .iter()
                    .position(|node| node.language == language && node.name == name),
            };

            nodes.extend(node);
        }

        for node in nodes {
            self.add(node, lock, true);
        }

        Ok(())
    }
}
