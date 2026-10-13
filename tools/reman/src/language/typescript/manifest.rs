//! `package.json`: its name, version and the dependencies inside the repository; and its version, written in
//! place.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use jsonc_parser::ParseOptions;
use jsonc_parser::cst::CstRootNode;
use serde_json::Value;

use crate::language::{Dependency, Package, Scope, Target};
use crate::paths::PathExt;

const DEPENDENCY_FIELDS: [(&str, Scope); 4] = [
    ("dependencies", Scope::Normal),
    ("peerDependencies", Scope::Normal),
    ("optionalDependencies", Scope::Normal),
    ("devDependencies", Scope::Dev),
];

pub struct PackageJson {
    /// Root-relative package folder.
    folder: PathBuf,
    json: Value,
}

impl PackageJson {
    pub const FILE_NAME: &str = "package.json";

    /// Manifest of `folder` (relative to `root`).
    pub fn read(root: &Path, folder: &Path) -> Result<Self> {
        let path = root.join(folder).join(Self::FILE_NAME);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let json =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;

        Ok(PackageJson {
            folder: folder.to_owned(),
            json,
        })
    }

    /// Package it declares, if it has a name; `overrides` (the workspace's, at `workspace`) link packages of
    /// other workspaces.
    pub fn package(
        self,
        workspace: &Path,
        overrides: &BTreeMap<String, String>,
    ) -> Option<Package> {
        let name = self.json.get("name")?.as_str()?.to_owned();

        Some(Package {
            name,
            version: self
                .json
                .get("version")
                .and_then(Value::as_str)
                .map(str::to_owned),
            dependencies: self.dependencies(workspace, overrides),
            references: Vec::new(),
            unpublished: self.json.get("private").and_then(Value::as_bool) == Some(true),
            binary: self.json.get("bin").is_some(),
            library: true,
            path: self.folder,
        })
    }

    /// Updates the top-level `"version"` in manifest `text` from `previous` to `version` through its syntax tree,
    /// preserving all other source text.
    pub fn with_version(text: &str, previous: &str, version: &str) -> Result<String> {
        let root = CstRootNode::parse(text, &ParseOptions::default())
            .map_err(|error| anyhow!("{error}"))?;
        let property = root.object_value().and_then(|object| object.get("version"));
        let current = property
            .as_ref()
            .and_then(|property| property.value())
            .and_then(|value| value.as_string_lit())
            .and_then(|literal| literal.decoded_value().ok());

        match property {
            Some(property) if current.as_deref() == Some(previous) => {
                property.set_value(version.into());

                Ok(root.to_string())
            }
            _ => bail!("no top-level \"version\": \"{previous}\" to replace"),
        }
    }

    /// Reads `workspace:` dependencies by name, `link:` and `file:` dependencies by path, and dependencies linked by `overrides`.
    fn dependencies(
        &self,
        workspace: &Path,
        overrides: &BTreeMap<String, String>,
    ) -> Vec<Dependency> {
        let mut dependencies = Vec::new();

        for (field, scope) in DEPENDENCY_FIELDS {
            for (name, specifier) in self
                .json
                .get(field)
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
            {
                let specifier = specifier.as_str().unwrap_or_default();
                let target = if let Some(link) = overrides
                    .get(name)
                    .and_then(|value| Self::local_path(value))
                {
                    Some(Target::Path(workspace.join(link).normalize()))
                } else if specifier.starts_with("workspace:") {
                    Some(Target::Name(name.clone()))
                } else {
                    Self::local_path(specifier)
                        .map(|link| Target::Path(self.folder.join(link).normalize()))
                };

                if let Some(target) = target {
                    dependencies.push(Dependency { target, scope });
                }
            }
        }

        dependencies
    }

    /// Folder a `link:` or `file:` specifier names.
    fn local_path(specifier: &str) -> Option<&str> {
        specifier
            .strip_prefix("link:")
            .or_else(|| specifier.strip_prefix("file:"))
    }
}
