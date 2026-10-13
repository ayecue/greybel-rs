//! Languages reman reads workspaces of. Each is a [`Language`]: how it finds its workspaces, reads their
//! packages, versions and dependencies, and writes a new version back. Nothing outside this module depends on
//! which language a workspace is; a new language is a new implementation in [`LanguageRegistry::ALL`].

pub mod dotnet;
pub mod duplex;
pub mod rust;
pub mod typescript;

mod files;
mod package;
mod registry;

use std::path::{Path, PathBuf};

use anyhow::Result;
pub use files::FileTree;
pub use package::{Dependency, LockChange, Package, Reference, Scope, Target, VersionChange};
pub use registry::LanguageRegistry;

use crate::edits::EditTable;

/// Version `reman init` gives a package without one.
pub const FIRST_VERSION: &str = "0.1.0";

/// Language version source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VersionSource {
    /// Manifest semver versions that reman updates and writes.
    Manifest,
    /// Names of its files (a new definition file is a new version): reman never writes them, and a change
    /// file's level is how far the version moved since the last release.
    FileNames,
}

pub trait Language: Sync {
    /// Name used by `language =` (`rust`).
    fn name(&self) -> &'static str;

    /// Project ID prefix (`rs`).
    fn prefix(&self) -> &'static str;

    fn version_source(&self) -> VersionSource {
        VersionSource::Manifest
    }

    /// Tool-output directories excluded from reman reads, such as `target`.
    fn build_folders(&self) -> &'static [&'static str] {
        &[]
    }

    /// Folders that look like its workspaces, among `files` (every file of the repository, relative to
    /// `root`), for `reman init`.
    fn find_workspaces(&self, root: &Path, files: &[PathBuf]) -> Vec<PathBuf>;

    /// Packages of the workspace at `workspace` (relative to `root`).
    fn packages(&self, root: &Path, workspace: &Path) -> Result<Vec<Package>>;

    /// Updates package manifests and workspace lock files for [`VersionSource::Manifest`] only.
    fn write_version(&self, change: &VersionChange<'_>, edits: &mut EditTable) -> Result<()>;

    /// Lock files of a workspace, relative to the root.
    fn lock_files(&self, _workspace: &Path) -> Vec<PathBuf> {
        Vec::new()
    }

    /// Packages whose resolved external dependencies differ between two versions of the lock file `lock`.
    fn lock_changes(&self, _lock: &Path, _before: &str, _after: &str) -> Result<Vec<LockChange>> {
        Ok(Vec::new())
    }

    /// File giving a package without a version its first one, for `reman init`.
    fn version_file(&self, _root: &Path, _package: &Package) -> Option<(PathBuf, String)> {
        None
    }
}
