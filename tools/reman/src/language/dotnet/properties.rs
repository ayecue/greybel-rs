//! Folder properties of `Directory.Build.props` files (`<GreybelDotnet>$([System.IO.Path]::GetFullPath(
//! '$(MSBuildThisFileDirectory)'))</GreybelDotnet>`), through which projects name files outside their folder.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Result, bail};
use regex::Regex;

use super::msbuild::MsBuildFile;
use crate::language::{Reference, Scope};
use crate::paths::PathExt;

/// Import depth limit of `Directory.Build.props` files.
const IMPORT_DEPTH: usize = 8;

/// Folder properties a project sees: name → folder, relative to the root.
#[derive(Default)]
pub struct FolderPropertyTable {
    folders: BTreeMap<String, PathBuf>,
}

impl FolderPropertyTable {
    /// Folder properties for a project in `directory`: from the closest `Directory.Build.props` above it and the files it
    /// imports.
    pub fn of(root: &Path, directory: &Path) -> Result<Self> {
        let mut properties = FolderPropertyTable::default();

        if let Some(props) = MsBuildFile::nearest_props(root, directory) {
            properties.read(root, &props, 0)?;
        }

        Ok(properties)
    }

    /// Paths `file` (at `source`) names through a folder property (`$(DuplexDotnet)duplex-core/src/
    /// DuplexCore.csproj`); a project file names its folder.
    pub fn references(
        &self,
        file: &MsBuildFile<'_>,
        scope: Scope,
        source: &Path,
    ) -> Vec<Reference> {
        static PROPERTY: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\$\((\w+)\)([^';*]*)").unwrap());

        file.expressions()
            .flat_map(|expression| PROPERTY.captures_iter(expression))
            .filter_map(|captures| {
                let folder = self.folders.get(&captures[1])?;
                let tail = captures[2].replace('\\', "/");
                let path = folder.join(tail.trim_start_matches('/')).normalize();
                let path = if MsBuildFile::is_project(&path) {
                    path.parent().unwrap_or(Path::new("")).to_path_buf()
                } else {
                    path
                };

                Some(Reference {
                    path,
                    scope,
                    source: source.to_owned(),
                })
            })
            .collect()
    }

    /// Merges folder properties from `props` and its imports at recursion depth `depth`.
    fn read(&mut self, root: &Path, props: &Path, depth: usize) -> Result<()> {
        static FULL_PATH: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\$\(\[System\.IO\.Path\]::GetFullPath\('\$\(MSBuildThisFileDirectory\)([^']*)'\)\)$").unwrap()
        });

        if depth > IMPORT_DEPTH {
            bail!("{}: imports nest past the depth limit", props.display());
        }

        let text = MsBuildFile::read(root, props)?;
        let file = MsBuildFile::parse(&text, props)?;
        let directory = props.parent().unwrap_or(Path::new(""));

        for project in file.attributes("Import", "Project") {
            let import = if project.contains("GetPathOfFileAbove") {
                directory
                    .parent()
                    .and_then(|parent| MsBuildFile::nearest_props(root, parent))
            } else if !project.contains("$(") {
                Some(directory.join(project.replace('\\', "/")).normalize())
            } else {
                None
            };

            if let Some(import) = import.filter(|import| root.join(import).exists()) {
                self.read(root, &import, depth + 1)?;
            }
        }

        for (name, value) in file.properties() {
            if let Some(captures) = FULL_PATH.captures(value) {
                self.folders
                    .insert(name.to_owned(), directory.join(&captures[1]).normalize());
            }
        }

        Ok(())
    }
}
