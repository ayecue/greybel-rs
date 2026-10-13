//! Duplex definitions: a folder of `<major>.<minor>.protocol` files is one package, its version the newest file's.
//! Workspace's folder is the package when it contains definition files itself (`duplex/core`), else each folder
//! below it that does (`protocols/io`). Versions are declared: a new version is a new file.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use super::{Language, Package, VersionChange, VersionSource};
use crate::edits::EditTable;

const EXTENSION: &str = "protocol";

pub struct Duplex;

impl Language for Duplex {
    fn name(&self) -> &'static str {
        "duplex"
    }

    fn prefix(&self) -> &'static str {
        "duplex"
    }

    fn version_source(&self) -> VersionSource {
        VersionSource::FileNames
    }

    /// Folder of definition folders (`protocols`), or a folder of definition files without siblings that have
    /// them (`duplex/core`).
    fn find_workspaces(&self, _root: &Path, files: &[PathBuf]) -> Vec<PathBuf> {
        let parent = |path: &Path| path.parent().map(Path::to_path_buf).unwrap_or_default();
        let folders = files
            .iter()
            .filter(|file| {
                file.extension()
                    .is_some_and(|extension| extension == EXTENSION)
            })
            .map(|file| parent(file))
            .collect::<BTreeSet<_>>();
        let mut workspaces = BTreeSet::new();

        for folder in &folders {
            let above = parent(folder);
            let siblings = folders
                .iter()
                .filter(|other| parent(other) == above)
                .count();

            workspaces.insert(if siblings > 1 { above } else { folder.clone() });
        }

        workspaces.into_iter().collect()
    }

    fn packages(&self, root: &Path, workspace: &Path) -> Result<Vec<Package>> {
        let folders = if Self::version_of(root, workspace)?.is_some() {
            vec![workspace.to_owned()]
        } else {
            let mut folders = Vec::new();

            for entry in std::fs::read_dir(root.join(workspace))? {
                let entry = entry?;

                if entry.file_type()?.is_dir() {
                    folders.push(workspace.join(entry.file_name()));
                }
            }

            folders.sort();

            folders
        };
        let mut packages = Vec::new();

        for folder in folders {
            let Some(version) = Self::version_of(root, &folder)? else {
                continue;
            };

            packages.push(Package {
                name: folder
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path: folder,
                version: Some(version),
                dependencies: Vec::new(),
                references: Vec::new(),
                unpublished: false,
                binary: false,
                library: true,
            });
        }

        Ok(packages)
    }

    fn write_version(&self, change: &VersionChange<'_>, _edits: &mut EditTable) -> Result<()> {
        bail!(
            "{}: a definition's version is its newest file; add {}.{EXTENSION} instead",
            change.package.path.display(),
            change.version
        )
    }
}

impl Duplex {
    /// Newest `<major>.<minor>` of the folder's definition files.
    fn version_of(root: &Path, folder: &Path) -> Result<Option<String>> {
        let mut newest: Option<(u64, u64)> = None;

        for entry in std::fs::read_dir(root.join(folder))? {
            let name = PathBuf::from(entry?.file_name());

            if name
                .extension()
                .is_none_or(|extension| extension != EXTENSION)
            {
                continue;
            }

            let Some((major, minor)) = name
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.split_once('.'))
            else {
                continue;
            };

            if let (Ok(major), Ok(minor)) = (major.parse(), minor.parse()) {
                newest = newest.max(Some((major, minor)));
            }
        }

        Ok(newest.map(|(major, minor)| format!("{major}.{minor}")))
    }
}
