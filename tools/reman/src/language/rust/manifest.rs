//! `Cargo.toml`, read: its package, version and path dependencies.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use toml::{Table, Value};

use crate::language::{Dependency, Scope, Target};
use crate::paths::PathExt;

const DEPENDENCY_TABLES: [(&str, Scope); 3] = [
    ("dependencies", Scope::Normal),
    ("build-dependencies", Scope::Normal),
    ("dev-dependencies", Scope::Dev),
];

pub struct CargoManifest {
    /// Root-relative package folder.
    pub folder: PathBuf,
    table: Table,
}

impl CargoManifest {
    pub const FILE_NAME: &str = "Cargo.toml";

    /// Manifest of `folder` (relative to `root`).
    pub fn read(root: &Path, folder: &Path) -> Result<Self> {
        let path = root.join(folder).join(Self::FILE_NAME);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let table = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;

        Ok(CargoManifest {
            folder: folder.to_owned(),
            table,
        })
    }

    /// Parsed `[workspace]` table.
    pub fn workspace(&self) -> Option<&Table> {
        self.table.get("workspace").and_then(Value::as_table)
    }

    fn package(&self) -> Option<&Table> {
        self.table.get("package").and_then(Value::as_table)
    }

    /// Package name from `[package]`, when present.
    pub fn name(&self) -> Result<Option<&str>> {
        let Some(package) = self.package() else {
            return Ok(None);
        };
        let name = package.get("name").and_then(Value::as_str);

        name.map(Some)
            .with_context(|| format!("{}: [package] without a name", self.folder.display()))
    }

    /// Package version, or `workspace_version` when `version.workspace = true`.
    pub fn version(&self, workspace_version: Option<&str>) -> Option<String> {
        match self.package()?.get("version")? {
            Value::String(version) => Some(version.clone()),
            Value::Table(version)
                if version.get("workspace").and_then(Value::as_bool) == Some(true) =>
            {
                workspace_version.map(str::to_owned)
            }
            _ => None,
        }
    }

    pub fn is_unpublished(&self) -> bool {
        matches!(
            self.package().and_then(|package| package.get("publish")),
            Some(Value::Boolean(false))
        )
    }

    /// Checks for an executable target through `[[bin]]` or `src/main.rs` relative to `root`.
    pub fn has_binary(&self, root: &Path) -> bool {
        self.table.contains_key("bin") || root.join(&self.folder).join("src/main.rs").exists()
    }

    /// Checks for a linkable library through `[lib]` or `src/lib.rs`.
    pub fn has_library(&self, root: &Path) -> bool {
        self.table.contains_key("lib") || root.join(&self.folder).join("src/lib.rs").exists()
    }

    /// Path dependencies of every dependency table, `[target.*]`'s included; `{ workspace = true }` ones
    /// through `workspace`'s `[workspace.dependencies]` (`workspace_folder` relative to the root).
    pub fn dependencies(
        &self,
        workspace_folder: &Path,
        workspace_dependencies: &Table,
    ) -> Vec<Dependency> {
        let targets = self
            .table
            .get("target")
            .and_then(Value::as_table)
            .into_iter()
            .flat_map(|targets| targets.values())
            .filter_map(Value::as_table);
        let mut dependencies = Vec::new();

        for table in std::iter::once(&self.table).chain(targets) {
            for (key, scope) in DEPENDENCY_TABLES {
                for (name, entry) in table
                    .get(key)
                    .and_then(Value::as_table)
                    .into_iter()
                    .flatten()
                {
                    let path = match entry {
                        Value::Table(entry)
                            if entry.get("workspace").and_then(Value::as_bool) == Some(true) =>
                        {
                            workspace_dependencies
                                .get(name)
                                .and_then(|dependency| dependency.get("path"))
                                .and_then(Value::as_str)
                                .map(|path| workspace_folder.join(path))
                        }
                        Value::Table(entry) => entry
                            .get("path")
                            .and_then(Value::as_str)
                            .map(|path| self.folder.join(path)),
                        _ => None,
                    };

                    if let Some(path) = path {
                        dependencies.push(Dependency {
                            target: Target::Path(path.normalize()),
                            scope,
                        });
                    }
                }
            }
        }

        dependencies
    }
}
