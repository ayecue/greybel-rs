//! Task as it applies to one place: the repository, a workspace (its language's defaults under its own) or a
//! project; and the step it becomes for a selection.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::{LINT, Mode, Selection, Step};
use crate::config::{PackageFilter, Select, TaskTable};
use crate::language::Language;
use crate::paths::PathExt;
use crate::repository::Repository;

/// Replacement for `{packages}` in a command.
const PACKAGES: &str = "{packages}";
/// Replacement for `{name}` in `select.each`.
const NAME: &str = "{name}";

/// File declaring a task: the root's, a workspace's or a project's.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Owner {
    Repository,
    Workspace(usize),
    Project(usize),
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub key: String,
    owner: Owner,
    pub table: TaskTable,
    /// Declaring file's folder, relative to the root.
    folder: PathBuf,
    language: Option<&'static dyn Language>,
    select: Option<Select>,
}

impl Instance {
    /// All repository tasks: root, workspaces in dependency order, then projects.
    pub fn all(repository: &Repository) -> Vec<Instance> {
        let mut instances = Vec::new();

        for (key, table) in &repository.tasks {
            instances.push(Instance {
                key: key.clone(),
                owner: Owner::Repository,
                table: table.clone(),
                folder: PathBuf::new(),
                language: table.language,
                select: table.select.clone(),
            });
        }

        for index in repository.workspace_order() {
            let workspace = &repository.workspaces[index];
            let mut tables = repository
                .languages
                .get(&workspace.language)
                .map(|language| language.tasks.clone())
                .unwrap_or_default();

            for (key, table) in &workspace.tasks {
                let task = tables
                    .get(key)
                    .map_or_else(|| table.clone(), |base: &TaskTable| base.merge(table));

                tables.insert(key.clone(), task);
            }

            for (key, table) in tables {
                let select = table
                    .select
                    .clone()
                    .or_else(|| repository.select(index).cloned());

                instances.push(Instance {
                    key,
                    owner: Owner::Workspace(index),
                    table,
                    folder: workspace.path.clone(),
                    language: Some(workspace.language),
                    select,
                });
            }
        }

        for index in repository.node_order() {
            let node = &repository.nodes[index];

            for (key, table) in &node.tasks {
                instances.push(Instance {
                    key: key.clone(),
                    owner: Owner::Project(index),
                    table: table.clone(),
                    folder: node.path().to_path_buf(),
                    language: Some(node.language),
                    select: None,
                });
            }
        }

        instances
    }

    /// Command `selection` runs of it, if it picks it.
    pub fn command(&self, selection: &Selection<'_>) -> Option<String> {
        let excluded = self.table.enabled == Some(false)
            || (!selection.tasks.is_empty() && !selection.tasks.contains(&self.key))
            || (!selection.languages.is_empty()
                && !self
                    .language
                    .is_some_and(|language| selection.languages.contains(&language)));

        if excluded {
            return None;
        }

        let suite = self.table.suite.as_deref().unwrap_or_default();

        match selection.mode {
            Mode::Run
                if selection
                    .suites
                    .iter()
                    .any(|suite_name| suite_name == suite) =>
            {
                self.table.run.clone()
            }
            Mode::Fix if suite == LINT => self.table.fix.clone(),
            _ => None,
        }
    }

    /// Step running `command`, unless `coverage` (packages to cover; none for everything) skips it.
    pub fn step(
        &self,
        repository: &Repository,
        command: String,
        coverage: Option<&BTreeSet<usize>>,
    ) -> Result<Option<Step>> {
        let cwd = self
            .folder
            .join(self.table.cwd.as_deref().unwrap_or(""))
            .normalize();
        let covers = |path: &Path| path.starts_with(&cwd) || cwd.starts_with(path);
        let label = self.label(repository);
        let command = match (self.owner, coverage) {
            (_, None) => self.with_packages(&label, command, None)?,
            (Owner::Repository, Some(coverage)) => {
                if !coverage
                    .iter()
                    .any(|node| covers(repository.nodes[*node].path()))
                {
                    return Ok(None);
                }

                command
            }
            (Owner::Workspace(index), Some(coverage)) => {
                let libraries_only = self.table.packages == Some(PackageFilter::Library);
                let names = coverage
                    .iter()
                    .map(|node| &repository.nodes[*node])
                    .filter(|node| node.workspace == index && covers(node.path()))
                    .filter(|node| !libraries_only || node.package.library)
                    .map(|node| node.name.as_str())
                    .collect::<Vec<_>>();

                if names.is_empty() {
                    return Ok(None);
                }

                self.with_packages(&label, command, Some(&names))?
            }
            (Owner::Project(index), Some(coverage)) => {
                if !coverage.contains(&index) {
                    return Ok(None);
                }

                command
            }
        };

        Ok(Some(Step {
            label,
            command,
            cwd,
            env: self.table.env.clone(),
        }))
    }

    /// Runs after tasks named `key`.
    pub fn runs_after(&self, key: &str) -> bool {
        self.key != key
            && self
                .table
                .after
                .as_ref()
                .is_some_and(|after| after.iter().any(|other| other == key))
    }

    fn label(&self, repository: &Repository) -> String {
        match self.owner {
            Owner::Repository => self.key.clone(),
            Owner::Workspace(index) => format!(
                "{}: {}",
                repository.workspaces[index].path.to_slash(),
                self.key
            ),
            Owner::Project(index) => format!("{}: {}", repository.nodes[index].id, self.key),
        }
    }

    /// Expands `{packages}` using `select.all`, or expands `select.each` separately for every entry in `names`.
    fn with_packages(
        &self,
        label: &str,
        command: String,
        names: Option<&[&str]>,
    ) -> Result<String> {
        if !command.contains(PACKAGES) {
            return Ok(command);
        }

        let select = self
            .select
            .as_ref()
            .with_context(|| format!("{label}: `{PACKAGES}` needs a `select`"))?;
        let packages = match names {
            None => select.all.clone(),
            Some(names) => names
                .iter()
                .map(|name| select.each.replace(NAME, name))
                .collect::<Vec<_>>()
                .join(" "),
        };

        Ok(command.replace(PACKAGES, &packages))
    }
}
