use std::collections::BTreeMap;
use std::path::Path;

use super::{FolderFailure, VersionsError};
use crate::model::Version;

const FILE: &str = "fingerprints.json";

/// Released definition versions mapped to binary-layout fingerprints.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FingerprintTable(BTreeMap<Version, u32>);

impl FingerprintTable {
    /// Loads recorded fingerprints from a definition folder.
    ///
    /// # Errors
    ///
    /// Returns an error if the fingerprint file cannot be read or parsed.
    pub fn read(folder: &Path) -> Result<Self, VersionsError> {
        let path = folder.join(FILE);
        let error = |failure| VersionsError::Folder {
            folder: folder.to_owned(),
            failure,
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(source) => return Err(error(FolderFailure::FingerprintRead(source))),
        };
        let entries: BTreeMap<String, u32> = serde_json::from_str(&text)
            .map_err(|source| error(FolderFailure::FingerprintMap(source)))?;

        entries
            .into_iter()
            .map(|(version, fingerprint)| {
                let parsed_version = version.parse().map_err(|()| {
                    error(FolderFailure::FingerprintVersion {
                        version: version.clone(),
                    })
                })?;

                Ok((parsed_version, fingerprint))
            })
            .collect::<Result<_, _>>()
            .map(Self)
    }

    #[must_use]
    pub fn get(&self, version: Version) -> Option<u32> {
        self.0.get(&version).copied()
    }

    pub fn versions(&self) -> impl Iterator<Item = Version> + '_ {
        self.0.keys().copied()
    }

    pub(crate) fn insert(&mut self, version: Version, fingerprint: u32) {
        self.0.entry(version).or_insert(fingerprint);
    }

    pub(crate) fn write(&self, folder: &Path) -> Result<(), VersionsError> {
        let entries: Vec<String> = self
            .0
            .iter()
            .map(|(version, fingerprint)| format!("  \"{version}\": {fingerprint}"))
            .collect();
        let text = format!("{{\n{}\n}}\n", entries.join(",\n"));

        std::fs::write(folder.join(FILE), text).map_err(|source| VersionsError::Folder {
            folder: folder.to_owned(),
            failure: FolderFailure::FingerprintWrite(source),
        })
    }
}
