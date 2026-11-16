//! Package repositories used by `aptclient`.

use greyhack_world_core::{ConnectionFailure, FileFailure, LibraryId, ServiceFailure};

/// Package published by a repository.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    /// Stored library installed to `/lib`, or `None` for a binary installed to `/usr/bin`.
    pub library: Option<LibraryId>,
}

/// Package repository identified by a domain such as `hackshop.org`.
#[derive(Debug, Clone, PartialEq)]
pub struct Repository {
    pub address: String,
    pub packages: Vec<Package>,
}

impl Repository {
    #[must_use]
    pub fn package(&self, name: &str) -> Option<&Package> {
        self.packages.iter().find(|package| package.name == name)
    }
}

/// Package returned by searches and repository listings. Name and description are `None` on the
/// official server, which omits both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEntry {
    pub name: String,
    pub description: Option<String>,
}

/// Errors returned by `aptclient` operations (`AptClientIntrinsics`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AptFailure {
    /// `/etc/apt/aptcache.bin` is missing because `update` has not run.
    NoCache,
    /// `/etc/apt/aptcache.bin` is not a binary cache produced by `update`.
    InvalidCache,
    /// The repository address must be an IP address.
    InvalidAddress(String),
    /// `sources.txt` already contains the repository.
    ListedRepository(String),
    /// `sources.txt` does not contain the repository.
    UnlistedRepository(String),
    /// No configured source matches the repository.
    UnknownRepository(String),
    /// No package from the source matches the search term.
    NoMatch(String),
    /// No configured source provides the package.
    UnknownPackage(String),
    /// The repository lists the package, but its server has no package file.
    MissingPackageFile(String),
    /// The computer no longer exists.
    ComputerUnavailable,
    /// The installation path does not identify a folder.
    MissingFolder(String),
    /// The package does not fit in the remaining disk space.
    NoSpace,
    /// The user cannot write to the installation folder.
    NoPermission,
    /// The package could not be copied from its server.
    Copy(FileFailure),
    /// The computer has no network access (`Computer.IsNetworkActive`).
    NoInternet,
    /// The package name is empty.
    EmptyPackageName,
    /// `sources.txt` is missing when adding or removing a repository.
    MissingSources,
    /// `sources.txt` is a binary file when adding or removing a repository.
    BinarySources,
    /// `sources.txt` contains malformed JSON.
    MalformedSources,
    /// `/etc/apt/aptcache.bin` is a binary an apt cache is not made of.
    MalformedCache,
    /// The file the library or program is checked at is not in the computer's file system.
    MissingLocalFile(String),
    /// The cache lists the server, which cannot be connected to.
    Connection(ConnectionFailure),
    /// The cache lists the server, whose repository service does not respond.
    Service(ServiceFailure),
    /// The server has no repository configuration file.
    MissingConfig,
    /// The server's repository configuration contains malformed JSON during installation.
    MalformedConfig,
    /// The server's repository configuration contains malformed JSON during an upgrade check; contains the package.
    MalformedConfigUpgrade(String),
    /// The server's repository folder does not exist.
    MissingRepositoryFolder,
}
