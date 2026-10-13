//! Workspaces the `reman.toml` files declare, and their packages.

use std::path::{Path, PathBuf};

use indexmap::IndexMap;

use crate::config::{ProjectBase, ProjectSettings, TaskTable};
use crate::language::{Language, Package};

pub struct Workspace {
    pub path: PathBuf,
    pub language: &'static dyn Language,
    /// Marks all packages development-only through `development = true`.
    pub development: bool,
    pub defaults: ProjectBase,
    /// Tasks its file declares (when it declares no project too).
    pub tasks: IndexMap<String, TaskTable>,
    pub origin: PathBuf,
}

impl Workspace {
    /// Innermost of `workspaces` containing `folder`.
    pub fn closest(workspaces: &[Workspace], folder: &Path) -> Option<usize> {
        workspaces
            .iter()
            .enumerate()
            .filter(|(_, workspace)| folder.starts_with(&workspace.path))
            .max_by_key(|(_, workspace)| workspace.path.components().count())
            .map(|(index, _)| index)
    }
}

/// Package of a declared workspace; a project when its folder has a `[project]`.
pub struct Node {
    /// Formats the ID as `<language prefix>/<name>`.
    pub id: String,
    pub name: String,
    pub language: &'static dyn Language,
    pub workspace: usize,
    pub package: Package,
    pub project: Option<ProjectSettings>,
    /// Project settings it would inherit (for packages without a file).
    pub defaults: ProjectBase,
    pub tasks: IndexMap<String, TaskTable>,
}

impl Node {
    pub fn path(&self) -> &Path {
        &self.package.path
    }

    pub fn is_project(&self) -> bool {
        self.project.is_some()
    }

    /// Checks for an app with GitHub tagging and releases enabled.
    pub fn is_released(&self) -> bool {
        self.project.as_ref().is_some_and(|project| project.release)
    }

    pub fn changelog(&self) -> PathBuf {
        let changelog = self
            .project
            .as_ref()
            .map_or(&self.defaults.changelog, |project| &project.changelog);

        self.package.path.join(&changelog.value)
    }

    pub fn skip(&self) -> &[String] {
        &self
            .project
            .as_ref()
            .map_or(&self.defaults.skip, |project| &project.skip)
            .value
    }
}
