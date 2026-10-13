mod additions;
mod comparison;
mod fingerprints;

use std::path::{Path, PathBuf};

pub use additions::AdditionTable;
use comparison::Comparison;
pub use fingerprints::FingerprintTable;

use crate::error::DefinitionError;
use crate::fingerprint::Fingerprint;
use crate::model::{Definition, Status, Version};

const EXTENSION: &str = "protocol";

/// Definition folder load, validation, or release error.
#[derive(Debug, thiserror::Error)]
pub enum FolderFailure {
    /// Failed to read the folder or an entry.
    #[error("can't be read: {0}")]
    Read(#[source] std::io::Error),
    /// Failed to read a definition file.
    #[error("can't read {stem}: {source}")]
    ReadVersion {
        stem: String,
        #[source]
        source: std::io::Error,
    },
    /// Definition file name is not a version.
    #[error("`{stem}.protocol` isn't named by its version, as `1.0.protocol`")]
    VersionName { stem: String },
    /// No definition file names the protocol.
    #[error("has no versions of `{name}`, as `1.0.protocol`")]
    NoVersions { name: String },
    /// Version does not follow its predecessor.
    #[error("has {version} where {next} comes")]
    VersionOrder { version: Version, next: Version },
    /// Released version binary layout differs.
    #[error("{version} changed its layout after it was released; give the change a new version")]
    LayoutChange { version: Version },
    /// A nonlatest version is unreleased.
    #[error("{version} isn't released: release it with `RELEASE_VERSIONS=1 cargo test`")]
    ReleaseMissing { version: Version },
    /// Fingerprint file names a version with no definition file.
    #[error("records {version}, which has no file")]
    UnknownFile { version: Version },
    /// Failed to read the fingerprint file.
    #[error("can't read fingerprints.json: {0}")]
    FingerprintRead(#[source] std::io::Error),
    /// Fingerprint file is not a version-to-fingerprint map.
    #[error("fingerprints.json isn't a map of versions to fingerprints: {0}")]
    FingerprintMap(#[source] serde_json::Error),
    /// Fingerprint file contains an invalid version.
    #[error("fingerprints.json records `{version}`, which is no version")]
    FingerprintVersion { version: String },
    /// Failed to write the fingerprint file.
    #[error("can't write fingerprints.json: {0}")]
    FingerprintWrite(#[source] std::io::Error),
}

/// Definition version load, validation, or release error.
#[derive(Debug, thiserror::Error)]
pub enum VersionsError {
    /// Folder I/O or version validation error.
    #[error("{}: {failure}", folder.display())]
    Folder {
        folder: PathBuf,
        #[source]
        failure: FolderFailure,
    },
    /// Definition file error.
    #[error("{}:{error}", file.display())]
    File {
        file: PathBuf,
        error: DefinitionError,
    },
}

/// Parsed definition and version metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct DefinitionVersion {
    pub version: Version,
    pub definition: Definition,
    /// Introducing minor version by declaration item, with 0 for initial major-version items.
    pub additions: AdditionTable,
    pub fingerprint: u32,
}

impl DefinitionVersion {
    #[must_use]
    pub fn fingerprint_of(&self, name: &str) -> Option<u32> {
        self.definition.declaration(name).map(|declaration| {
            Fingerprint::of_type(&self.definition, &self.additions, &declaration.name)
        })
    }
}

/// Definition versions sorted by major and minor.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionHistory {
    name: String,
    versions: Vec<DefinitionVersion>,
}

/// Borrowed minor versions sorted within one major.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinorVersionTable<'v> {
    versions: &'v [DefinitionVersion],
}

impl VersionHistory {
    /// Loads and validates every definition version in a folder.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid files, versions, or recorded fingerprints.
    pub fn load(folder: &Path) -> Result<Self, VersionsError> {
        let versions = Self::read(folder)?;
        let fingerprints = FingerprintTable::read(folder)?;
        let name = folder
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let versions = Self::build(folder, &name, versions)?;
        let versions = Self { name, versions };

        versions.validate(folder, &fingerprints, true)?;

        Ok(versions)
    }

    /// Validates a definition folder, records fingerprints for unreleased versions, and returns newly released
    /// versions.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid folder or failed fingerprint update.
    pub fn release(folder: &Path) -> Result<Vec<Version>, VersionsError> {
        let mut fingerprints = FingerprintTable::read(folder)?;
        let name = folder
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let versions = Self::build(folder, &name, Self::read(folder)?)?;
        let versions = Self { name, versions };

        versions.validate(folder, &fingerprints, false)?;

        let new_versions: Vec<Version> = versions
            .versions
            .iter()
            .filter(|version| fingerprints.get(version.version).is_none())
            .map(|version| version.version)
            .collect();

        if !new_versions.is_empty() {
            for version in &versions.versions {
                fingerprints.insert(version.version, version.fingerprint);
            }

            fingerprints.write(folder)?;
        }

        Ok(new_versions)
    }

    /// Builds version history from memory without fingerprint I/O.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid source or a minor that changes an existing layout.
    pub fn of_sources(name: &str, sources: &[(Version, &str)]) -> Result<Self, VersionsError> {
        let folder = Path::new(name);
        let mut versions = Vec::new();

        for (version, source) in sources {
            let file = Self::file(folder, *version);
            let definition =
                Definition::parse(source).map_err(|error| VersionsError::File { file, error })?;

            versions.push((*version, definition));
        }

        versions.sort_by_key(|(version, _)| *version);

        Ok(Self {
            name: name.to_owned(),
            versions: Self::build(folder, name, versions)?,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn all(&self) -> &[DefinitionVersion] {
        &self.versions
    }

    #[must_use]
    pub fn latest(&self) -> Option<&DefinitionVersion> {
        self.versions.last()
    }

    #[must_use]
    pub fn major(&self, major: u16) -> Option<MinorVersionTable<'_>> {
        let start = self
            .versions
            .iter()
            .position(|version| version.version.major == major)?;
        let end = self.versions[start..]
            .iter()
            .position(|version| version.version.major != major)
            .map_or(self.versions.len(), |end| start + end);

        Some(MinorVersionTable {
            versions: &self.versions[start..end],
        })
    }

    fn read(folder: &Path) -> Result<Vec<(Version, Definition)>, VersionsError> {
        let folder_error = |failure| VersionsError::Folder {
            folder: folder.to_owned(),
            failure,
        };
        let entries = std::fs::read_dir(folder)
            .map_err(|source| folder_error(FolderFailure::Read(source)))?;
        let mut versions = Vec::new();

        for entry in entries {
            let path = entry
                .map_err(|source| folder_error(FolderFailure::Read(source)))?
                .path();

            if path
                .extension()
                .is_none_or(|extension| extension != EXTENSION)
            {
                continue;
            }

            let stem = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            let version: Version = stem
                .parse()
                .map_err(|()| folder_error(FolderFailure::VersionName { stem: stem.clone() }))?;
            let source = std::fs::read_to_string(&path)
                .map_err(|source| folder_error(FolderFailure::ReadVersion { stem, source }))?;
            let definition = Definition::parse(&source)
                .map_err(|error| VersionsError::File { file: path, error })?;

            versions.push((version, definition));
        }

        versions.sort_by_key(|(version, _)| *version);

        Ok(versions)
    }

    fn build(
        folder: &Path,
        name: &str,
        versions: Vec<(Version, Definition)>,
    ) -> Result<Vec<DefinitionVersion>, VersionsError> {
        if versions.is_empty() {
            return Err(VersionsError::Folder {
                folder: folder.to_owned(),
                failure: FolderFailure::NoVersions {
                    name: name.to_owned(),
                },
            });
        }

        let mut ordered_versions: Vec<DefinitionVersion> = Vec::with_capacity(versions.len());

        for (version, definition) in versions {
            let previous = ordered_versions.last().map(|previous| previous.version);
            let expected_version = match previous {
                Some(previous) if previous.major == version.major => {
                    Version::new(version.major, previous.minor + 1)
                }
                _ => Version::new(version.major, 0),
            };

            if version != expected_version {
                return Err(VersionsError::Folder {
                    folder: folder.to_owned(),
                    failure: FolderFailure::VersionOrder {
                        version,
                        next: expected_version,
                    },
                });
            }

            let additions = match ordered_versions
                .last()
                .filter(|previous| previous.version.major == version.major)
            {
                None => AdditionTable::default(),
                Some(previous) => Comparison::additions(
                    &previous.definition,
                    &previous.additions,
                    &definition,
                    version.minor,
                )
                .map_err(|error| VersionsError::File {
                    file: Self::file(folder, version),
                    error: DefinitionError::new(
                        error.span,
                        format!(
                            "{}; only a new major, {}.0, may",
                            error.message,
                            version.major + 1
                        ),
                    ),
                })?,
            };
            let fingerprint = Fingerprint::of(&definition, &additions);

            ordered_versions.push(DefinitionVersion {
                version,
                definition,
                additions,
                fingerprint,
            });
        }

        Ok(ordered_versions)
    }

    fn validate(
        &self,
        folder: &Path,
        fingerprints: &FingerprintTable,
        released: bool,
    ) -> Result<(), VersionsError> {
        let folder_error = |failure| VersionsError::Folder {
            folder: folder.to_owned(),
            failure,
        };

        for (index, version) in self.versions.iter().enumerate() {
            match fingerprints.get(version.version) {
                Some(fingerprint) if fingerprint != version.fingerprint => {
                    return Err(folder_error(FolderFailure::LayoutChange {
                        version: version.version,
                    }));
                }
                None if released && index + 1 < self.versions.len() => {
                    return Err(folder_error(FolderFailure::ReleaseMissing {
                        version: version.version,
                    }));
                }
                _ => {}
            }
        }

        if let Some(stray) = fingerprints.versions().find(|version| {
            !self
                .versions
                .iter()
                .any(|candidate| candidate.version == *version)
        }) {
            return Err(folder_error(FolderFailure::UnknownFile { version: stray }));
        }

        Ok(())
    }

    fn file(folder: &Path, version: Version) -> PathBuf {
        folder.join(format!("{version}.{EXTENSION}"))
    }
}

impl<'v> MinorVersionTable<'v> {
    #[must_use]
    pub fn major(&self) -> u16 {
        self.latest().version.major
    }

    #[must_use]
    pub fn latest(&self) -> &'v DefinitionVersion {
        &self.versions[self.versions.len() - 1]
    }

    #[must_use]
    pub fn all(&self) -> &'v [DefinitionVersion] {
        self.versions
    }

    #[must_use]
    pub fn status(&self, minor: u16) -> Option<&'v Status> {
        self.versions
            .get(usize::from(minor))
            .map(|version| version.definition.status())
    }
}
