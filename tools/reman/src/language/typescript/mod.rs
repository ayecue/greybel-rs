//! pnpm workspace support for package globs and `link:` overrides in `pnpm-workspace.yaml`, `package.json`
//! versions and dependencies, and `pnpm-lock.yaml`.

mod lock;
mod manifest;
mod workspace;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lock::PnpmLock;
use manifest::PackageJson;
use workspace::PnpmWorkspace;

use super::{Language, LockChange, Package, VersionChange};
use crate::edits::EditTable;

pub struct Typescript;

impl Language for Typescript {
    fn name(&self) -> &'static str {
        "typescript"
    }

    fn prefix(&self) -> &'static str {
        "ts"
    }

    fn build_folders(&self) -> &'static [&'static str] {
        &["node_modules"]
    }

    fn find_workspaces(&self, _root: &Path, files: &[PathBuf]) -> Vec<PathBuf> {
        files
            .iter()
            .filter(|file| {
                file.file_name()
                    .is_some_and(|name| name == PnpmWorkspace::FILE_NAME)
            })
            .map(|file| file.parent().unwrap_or(Path::new("")).to_path_buf())
            .collect()
    }

    fn packages(&self, root: &Path, workspace: &Path) -> Result<Vec<Package>> {
        PnpmWorkspace::read(root, workspace)?.packages(root)
    }

    fn write_version(&self, change: &VersionChange<'_>, edits: &mut EditTable) -> Result<()> {
        let manifest = change.package.path.join(PackageJson::FILE_NAME);
        let text = edits
            .read(&manifest)?
            .with_context(|| format!("{} is missing", manifest.display()))?;
        let text = PackageJson::with_version(&text, change.previous, change.version)
            .with_context(|| manifest.display().to_string())?;

        edits.write(&manifest, text);

        Ok(())
    }

    fn lock_files(&self, workspace: &Path) -> Vec<PathBuf> {
        vec![workspace.join(PnpmLock::FILE_NAME)]
    }

    fn lock_changes(&self, lock: &Path, before: &str, after: &str) -> Result<Vec<LockChange>> {
        Ok(PnpmLock::parse(before)?.changes_to(&PnpmLock::parse(after)?, lock))
    }
}
