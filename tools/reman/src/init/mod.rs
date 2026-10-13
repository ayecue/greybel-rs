//! Creates missing `reman.toml` files from reviewable guesses. Each detected workspace receives `[workspace]`;
//! each published package receives `[project]`, using `--kind`, otherwise `app` for programs or `library`.
//! `--dev` marks development-only packages and workspace folders. Subsequent runs create only missing files.

mod matcher;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use jiff::civil::Date;
use matcher::KindTable;

use crate::changes::ReleaseRecord;
use crate::config::{self, File, Kind, ProjectTable, RepositoryTable, WorkspaceTable};
use crate::edits::EditTable;
use crate::git::Git;
use crate::language::{FIRST_VERSION, FileTree, LanguageRegistry};
use crate::release::changelog::Changelog;
use crate::repository::Repository;

/// `github` of a repository whose `origin` remote isn't on GitHub.
const UNKNOWN_GITHUB: &str = "owner/name";

pub struct InitOptions {
    /// First release record's date.
    pub date: Date,
    /// Maps each pattern to a project kind; earliest match wins.
    pub kinds: Vec<KindPattern>,
    /// Selects development-only packages and workspace folders.
    pub dev: Vec<String>,
}

/// `KIND=PATTERN`: assigns that kind to packages selected by ID, name, or folder glob.
#[derive(Clone, Debug)]
pub struct KindPattern {
    pub kind: Kind,
    pub pattern: String,
}

impl FromStr for KindPattern {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self> {
        let (kind, pattern) = text.split_once('=').context("expected KIND=PATTERN")?;

        Ok(KindPattern {
            kind: Kind::parse(kind)?,
            pattern: pattern.to_owned(),
        })
    }
}

pub struct InitOutcome {
    pub edits: EditTable,
    pub report: String,
}

/// Initializes a repository in one `reman init` run.
pub struct Initializer<'a> {
    root: &'a Path,
    options: &'a InitOptions,
    rules: KindTable,
    /// Root-relative folders that already contain `reman.toml`.
    present: BTreeSet<PathBuf>,
    /// Files to write, by folder.
    files: BTreeMap<PathBuf, File>,
    /// Additional edits (initial versions and release record).
    edits: EditTable,
    /// Packages without a `[project]` declaration.
    dev_packages: Vec<String>,
}

impl<'a> Initializer<'a> {
    pub fn run(root: &'a Path, options: &'a InitOptions) -> Result<InitOutcome> {
        let tree = FileTree::new(root).files(Path::new(""), |_| true);
        let present = tree
            .iter()
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| name == config::FILE_NAME)
            })
            .map(|path| path.parent().map(Path::to_path_buf).unwrap_or_default())
            .collect();
        let mut initializer = Initializer {
            root,
            options,
            rules: KindTable::new(options)?,
            present,
            files: BTreeMap::new(),
            edits: EditTable::new(root),
            dev_packages: Vec::new(),
        };

        initializer.add_repository();
        initializer.add_workspaces(&tree);

        let repository = Repository::load_with(root, &initializer.file_texts()?)?;

        initializer.add_projects(&repository);
        initializer.drop_dev_workspaces(&repository);

        for (path, text) in initializer.file_texts()? {
            initializer.edits.write(&path, text);
        }

        initializer.add_record(&repository)?;
        initializer.add_changelogs()?;

        Ok(initializer.outcome())
    }

    /// Root's `[repository]`, `github` from the `origin` remote.
    fn add_repository(&mut self) {
        if self.present.contains(Path::new("")) {
            return;
        }

        let github = Git::open(self.root)
            .ok()
            .and_then(|git| git.github_repository())
            .unwrap_or_else(|| UNKNOWN_GITHUB.to_owned());

        self.files.entry(PathBuf::new()).or_default().repository =
            Some(RepositoryTable::new(github));
    }

    /// `[workspace]` for each workspace a language finds among the repository's files.
    fn add_workspaces(&mut self, tree: &[PathBuf]) {
        for language in LanguageRegistry::ALL {
            for folder in language.find_workspaces(self.root, tree) {
                if !self.present.contains(&folder) && !self.rules.is_dev_folder(&folder) {
                    let workspace = WorkspaceTable {
                        language,
                        development: false,
                        project: None,
                    };

                    self.files.entry(folder).or_default().workspace = Some(workspace);
                }
            }
        }
    }

    /// `[project]` for each package without a file that isn't for development only (its workspace's
    /// `development`, `--dev`, or its manifest says so), and its first version.
    fn add_projects(&mut self, repository: &Repository) {
        for node in repository
            .nodes
            .iter()
            .filter(|node| !self.present.contains(node.path()))
        {
            let kind = if repository.workspaces[node.workspace].development {
                None
            } else {
                self.rules.kind_of(node)
            };
            let Some(kind) = kind else {
                self.dev_packages.push(node.id.clone());
                continue;
            };

            // Workspace that is its own package (`duplex/core`) declares both in one file.
            self.files
                .entry(node.path().to_path_buf())
                .or_default()
                .project = Some(ProjectTable::new(kind));

            if let Some((path, text)) = node.language.version_file(self.root, &node.package) {
                self.edits.write(&path, text);
            }
        }
    }

    /// New workspace of development packages only needs no file.
    fn drop_dev_workspaces(&mut self, repository: &Repository) {
        let with_projects = repository
            .nodes
            .iter()
            .filter(|node| {
                node.is_project()
                    || self
                        .files
                        .get(node.path())
                        .is_some_and(|file| file.project.is_some())
            })
            .map(|node| repository.workspaces[node.workspace].path.clone())
            .collect::<BTreeSet<_>>();

        self.files
            .retain(|folder, file| file.workspace.is_none() || with_projects.contains(folder));
    }

    /// First release record: every project's version as it is.
    fn add_record(&mut self, repository: &Repository) -> Result<()> {
        let path = repository.changes().record_path();

        if self.root.join(&path).exists() {
            return Ok(());
        }

        let repository = Repository::load_with(self.root, &self.edits.writes())?;
        let mut record = ReleaseRecord {
            date: self.options.date,
            ..ReleaseRecord::default()
        };

        for (_, node) in repository.projects() {
            let version = node
                .package
                .version
                .clone()
                .unwrap_or_else(|| FIRST_VERSION.to_owned());

            record.versions.insert(node.id.clone(), version);
        }

        self.edits.write(&path, record.render()?);

        Ok(())
    }

    /// Empty changelog for each project without one, those this run declares included.
    fn add_changelogs(&mut self) -> Result<()> {
        let repository = Repository::load_with(self.root, &self.edits.writes())?;

        for (_, node) in repository.projects() {
            let path = node.changelog();

            if !self.root.join(&path).exists() {
                self.edits.write(&path, Changelog::empty(&node.name));
            }
        }

        Ok(())
    }

    /// Files to write as `(path, text)` pairs.
    fn file_texts(&self) -> Result<Vec<(PathBuf, String)>> {
        self.files
            .iter()
            .map(|(folder, file)| Ok((folder.join(config::FILE_NAME), file.render()?)))
            .collect()
    }

    fn outcome(self) -> InitOutcome {
        let mut report = String::new();

        if !self.dev_packages.is_empty() {
            let _ = writeln!(
                report,
                "No [project] (development only), review: {}",
                self.dev_packages.join(", ")
            );
        }

        InitOutcome {
            edits: self.edits,
            report,
        }
    }
}
