//! .NET package: a folder of a workspace with its projects, which ship together.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::msbuild::MsBuildFile;
use super::properties::FolderPropertyTable;
use crate::language::{FileTree, Package, Scope};

/// `OutputType`s of a project building a program.
const PROGRAM_OUTPUTS: [&str; 2] = ["Exe", "WinExe"];

pub struct PackageFolder {
    /// Root-relative path.
    pub path: PathBuf,
    /// Root-relative paths to its project files.
    projects: Vec<PathBuf>,
}

impl PackageFolder {
    /// Folders of `workspace` (relative to `root`) with projects in or below them.
    pub fn all(root: &Path, workspace: &Path) -> Vec<Self> {
        let mut folders = BTreeMap::<PathBuf, Vec<PathBuf>>::new();

        for project in FileTree::new(root).files(workspace, MsBuildFile::is_project) {
            let Ok(inside) = project.strip_prefix(workspace) else {
                continue;
            };

            if let (Some(first), true) =
                (inside.components().next(), inside.components().count() > 1)
            {
                folders
                    .entry(workspace.join(first))
                    .or_default()
                    .push(project);
            }
        }

        folders
            .into_iter()
            .map(|(path, projects)| PackageFolder { path, projects })
            .collect()
    }

    /// Package: its projects' references (those of test projects for tests only), whether one builds a program,
    /// and unpublished when each of its projects is a test project or sets `<IsPackable>false</IsPackable>`.
    pub fn package(self, root: &Path) -> Result<Package> {
        let mut dependencies = Vec::new();
        let mut references = Vec::new();
        let mut binary = false;
        let mut unpublished = true;

        for project in &self.projects {
            let text = MsBuildFile::read(root, project)?;
            let file = MsBuildFile::parse(&text, project)?;
            let directory = project.parent().unwrap_or(Path::new(""));
            let scope = self.scope_of(project, &file);

            unpublished &= scope == Scope::Dev
                || file
                    .values("IsPackable")
                    .any(|value| value.eq_ignore_ascii_case("false"));
            binary |= file
                .values("OutputType")
                .any(|value| PROGRAM_OUTPUTS.contains(&value));
            dependencies.extend(file.project_references(directory, scope));
            references.extend(
                FolderPropertyTable::of(root, directory)?.references(&file, scope, project),
            );
        }

        Ok(Package {
            name: self.name(root)?,
            version: Self::version(root, &self.path)?,
            dependencies,
            references,
            unpublished,
            binary,
            library: true,
            path: self.path,
        })
    }

    /// Files of the package at `folder` that may contain its version.
    pub fn version_files(root: &Path, folder: &Path) -> Vec<PathBuf> {
        FileTree::new(root).files(folder, |path| {
            MsBuildFile::is_project(path)
                || path
                    .file_name()
                    .is_some_and(|name| name == MsBuildFile::DIRECTORY_PROPS)
        })
    }

    /// First `<Version>` of its files.
    fn version(root: &Path, folder: &Path) -> Result<Option<String>> {
        for file in Self::version_files(root, folder) {
            let text = MsBuildFile::read(root, &file)?;

            if let Some(version) = MsBuildFile::parse(&text, &file)?.values("Version").next() {
                return Ok(Some(version.to_owned()));
            }
        }

        Ok(None)
    }

    /// Test project (below `tests/`, or `IsTestProject`) reaches only the package's tests.
    fn scope_of(&self, project: &Path, file: &MsBuildFile<'_>) -> Scope {
        let inside = project.strip_prefix(&self.path).unwrap_or(project);
        let test = inside
            .components()
            .any(|component| component.as_os_str() == "tests")
            || file
                .values("IsTestProject")
                .any(|value| value.eq_ignore_ascii_case("true"));

        if test { Scope::Dev } else { Scope::Normal }
    }

    /// Package name from its solution (the folder's only `.slnx` or `.sln`), otherwise its only project,
    /// or the folder.
    fn name(&self, root: &Path) -> Result<String> {
        let mut solutions = Vec::new();

        for entry in std::fs::read_dir(root.join(&self.path))? {
            let path = PathBuf::from(entry?.file_name());

            if path
                .extension()
                .is_some_and(|extension| extension == "slnx" || extension == "sln")
            {
                solutions.push(path);
            }
        }

        let stem = |path: &Path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        };
        let name = match (solutions.as_slice(), self.projects.as_slice()) {
            ([solution], _) => stem(solution),
            (_, [project]) => stem(project),
            _ => self
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned()),
        };

        Ok(name.unwrap_or_default())
    }
}
