//! Manifest data: package definitions, dependencies, read paths, and data
//! they write back.

use std::path::PathBuf;

/// Package as its workspace's manifests declare it.
#[derive(Clone, Debug)]
pub struct Package {
    pub name: String,
    /// Root-relative package folder.
    pub path: PathBuf,
    pub version: Option<String>,
    pub dependencies: Vec<Dependency>,
    /// External folders and files read through definition macros or MSBuild properties.
    pub references: Vec<Reference>,
    /// Indicates non-publication through `publish = false`, `"private": true`, or test-only projects.
    pub unpublished: bool,
    /// Indicates that the package produces an executable.
    pub binary: bool,
    /// Indicates a linkable library; false only for Rust crates without a `lib` target.
    pub library: bool,
}

/// Dependency scope: shipped package or tests only.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Scope {
    Normal,
    Dev,
}

#[derive(Clone, Debug)]
pub struct Dependency {
    pub target: Target,
    pub scope: Scope,
}

#[derive(Clone, Debug)]
pub enum Target {
    /// Folder, relative to the root.
    Path(PathBuf),
    /// Package of the same workspace.
    Name(String),
}

/// Path a package names through the repository's folder table (`"$PROTOCOLS/io"`, `$(DuplexDotnet)`).
#[derive(Clone, Debug)]
pub struct Reference {
    /// Root-relative path.
    pub path: PathBuf,
    pub scope: Scope,
    /// File naming it, relative to the root.
    pub source: PathBuf,
}

/// Package's new version, to write over its previous one.
pub struct VersionChange<'a> {
    pub package: &'a Package,
    pub previous: &'a str,
    pub version: &'a str,
    /// Root-relative workspaces of the package's language whose lock files list it.
    pub workspaces: &'a [PathBuf],
}

/// Package whose resolved external dependencies differ between two versions of a lock file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LockChange {
    /// Identifies the change by root-relative folder.
    Folder(PathBuf),
    /// Identifies the package by name within its language.
    Package(String),
}
