//! Cargo workspace: its members, the version and dependencies they inherit, and the packages they are.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use toml::{Table, Value};

use super::environment::Environment;
use super::manifest::CargoManifest;
use crate::language::{FileTree, Package};
use crate::paths::{GlobMatcher, PathExt};

pub struct CargoWorkspace {
    /// Root-relative package folder.
    folder: PathBuf,
    table: Table,
}

impl CargoWorkspace {
    /// Workspace of `folder` (relative to `root`).
    pub fn read(root: &Path, folder: &Path) -> Result<Self> {
        let manifest = CargoManifest::read(root, folder)?;
        let Some(table) = manifest.workspace() else {
            bail!(
                "{}/{} has no [workspace]",
                folder.display(),
                CargoManifest::FILE_NAME
            );
        };

        Ok(CargoWorkspace {
            folder: folder.to_owned(),
            table: table.clone(),
        })
    }

    /// Workspace members representing packages.
    pub fn packages(&self, root: &Path) -> Result<Vec<Package>> {
        let version = self
            .table
            .get("package")
            .and_then(|package| package.get("version"))
            .and_then(Value::as_str);
        let dependencies = self
            .table
            .get("dependencies")
            .and_then(Value::as_table)
            .cloned()
            .unwrap_or_default();
        let environment = Environment::load(root)?;
        let mut packages = Vec::new();

        for member in self.members(root)? {
            let manifest = CargoManifest::read(root, &member)?;

            let Some(name) = manifest.name()? else {
                continue;
            };

            packages.push(Package {
                name: name.to_owned(),
                version: manifest.version(version),
                dependencies: manifest.dependencies(&self.folder, &dependencies),
                references: environment.sources_of(root, &member)?,
                unpublished: manifest.is_unpublished(),
                binary: manifest.has_binary(root),
                library: manifest.has_library(root),
                path: member,
            });
        }

        Ok(packages)
    }

    /// Member folders `members` matches and `exclude` doesn't, relative to the root.
    fn members(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let patterns = self.strings("members");
        let exclusions = self.strings("exclude");
        let globs = GlobMatcher::new(patterns.iter().map(|pattern| pattern.trim_end_matches('/')))?;
        let mut members = BTreeSet::new();

        if patterns.iter().any(|pattern| pattern == ".") {
            members.insert(self.folder.clone());
        }

        let manifests = FileTree::new(root).files(&self.folder, |path| {
            path.file_name()
                .is_some_and(|name| name == CargoManifest::FILE_NAME)
        });

        for manifest in manifests {
            let folder = manifest.parent().unwrap_or(Path::new("")).to_path_buf();
            let Ok(inside) = folder.strip_prefix(&self.folder) else {
                continue;
            };
            let inside = inside.to_slash();

            if !inside.is_empty() && globs.matches(&inside) && !exclusions.contains(&inside) {
                members.insert(folder);
            }
        }

        Ok(members.into_iter().collect())
    }

    fn strings(&self, key: &str) -> Vec<String> {
        self.table
            .get(key)
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }
}
