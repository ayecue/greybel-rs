//! Cargo workspace support: members, path dependencies, versions, `Cargo.lock`, and folder paths from the root
//! `.cargo/config.toml`. Released crates replace `version.workspace = true` with explicit versions.

mod environment;
mod lock;
mod manifest;
mod workspace;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
pub use environment::Environment;
use lock::CargoLock;
use manifest::CargoManifest;
use workspace::CargoWorkspace;

use super::{Language, LockChange, Package, VersionChange};
use crate::edits::EditTable;

pub struct Rust;

impl Language for Rust {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn prefix(&self) -> &'static str {
        "rs"
    }

    fn build_folders(&self) -> &'static [&'static str] {
        &["target"]
    }

    fn find_workspaces(&self, root: &Path, files: &[PathBuf]) -> Vec<PathBuf> {
        files
            .iter()
            .filter(|file| {
                file.file_name()
                    .is_some_and(|name| name == CargoManifest::FILE_NAME)
            })
            .map(|file| file.parent().unwrap_or(Path::new("")).to_path_buf())
            .filter(|folder| {
                CargoManifest::read(root, folder)
                    .is_ok_and(|manifest| manifest.workspace().is_some())
            })
            .collect()
    }

    fn packages(&self, root: &Path, workspace: &Path) -> Result<Vec<Package>> {
        CargoWorkspace::read(root, workspace)?.packages(root)
    }

    fn write_version(&self, change: &VersionChange<'_>, edits: &mut EditTable) -> Result<()> {
        let manifest = change.package.path.join(CargoManifest::FILE_NAME);
        let text = edits
            .read(&manifest)?
            .with_context(|| format!("{} is missing", manifest.display()))?;
        let mut document: toml_edit::DocumentMut = text.parse()?;

        document["package"]["version"] = toml_edit::value(change.version);
        edits.write(&manifest, document.to_string());

        for lock in change
            .workspaces
            .iter()
            .flat_map(|workspace| self.lock_files(workspace))
        {
            if let Some(text) = edits.read(&lock)?
                && let Some(text) = CargoLock::new(&text).with_version(
                    &change.package.name,
                    change.previous,
                    change.version,
                )?
            {
                edits.write(&lock, text);
            }
        }

        Ok(())
    }

    fn lock_files(&self, workspace: &Path) -> Vec<PathBuf> {
        vec![workspace.join(CargoLock::FILE_NAME)]
    }

    fn lock_changes(&self, _lock: &Path, before: &str, after: &str) -> Result<Vec<LockChange>> {
        CargoLock::new(before).changes_to(&CargoLock::new(after))
    }
}
