//! Pnpm workspace: `pnpm-workspace.yaml`'s package globs and `overrides`, and the packages they match.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use super::manifest::PackageJson;
use crate::language::{FileTree, Package};
use crate::paths::{GlobMatcher, PathExt};

pub struct PnpmWorkspace {
    /// Root-relative package folder.
    folder: PathBuf,
    inclusions: GlobMatcher,
    exclusions: GlobMatcher,
    /// External packages linked with specifiers such as `link:../../common/ts/<package>`.
    overrides: BTreeMap<String, String>,
}

#[derive(Default, Deserialize)]
struct WorkspaceFile {
    #[serde(default)]
    packages: Vec<String>,
    #[serde(default)]
    overrides: BTreeMap<String, String>,
}

impl PnpmWorkspace {
    pub const FILE_NAME: &str = "pnpm-workspace.yaml";

    /// Workspace of `folder` (relative to `root`).
    pub fn read(root: &Path, folder: &Path) -> Result<Self> {
        let path = root.join(folder).join(Self::FILE_NAME);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let file: WorkspaceFile = serde_yaml_ng::from_str(&text)
            .with_context(|| format!("parsing {}", path.display()))?;
        let (exclusions, inclusions): (Vec<&String>, Vec<&String>) = file
            .packages
            .iter()
            .partition(|pattern| pattern.starts_with('!'));

        Ok(PnpmWorkspace {
            folder: folder.to_owned(),
            inclusions: GlobMatcher::new(inclusions)?,
            exclusions: GlobMatcher::new(exclusions.iter().map(|pattern| &pattern[1..]))?,
            overrides: file.overrides,
        })
    }

    /// Packages of the folders its globs match.
    pub fn packages(&self, root: &Path) -> Result<Vec<Package>> {
        let manifests = FileTree::new(root).files(&self.folder, |path| {
            path.file_name()
                .is_some_and(|name| name == PackageJson::FILE_NAME)
        });
        let mut packages = Vec::new();

        for manifest in manifests {
            let folder = manifest.parent().unwrap_or(Path::new("")).to_path_buf();

            if self.contains(&folder)
                && let Some(package) =
                    PackageJson::read(root, &folder)?.package(&self.folder, &self.overrides)
            {
                packages.push(package);
            }
        }

        Ok(packages)
    }

    /// Checks whether workspace package globs match root-relative `folder`.
    fn contains(&self, folder: &Path) -> bool {
        let inside = folder
            .strip_prefix(&self.folder)
            .unwrap_or(folder)
            .to_slash();

        !inside.is_empty() && self.inclusions.matches(&inside) && !self.exclusions.matches(&inside)
    }
}
