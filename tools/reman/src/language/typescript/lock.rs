//! Detects importers with changed resolved dependencies between two `pnpm-lock.yaml` versions.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::language::LockChange;
use crate::paths::PathExt;

#[derive(Deserialize)]
struct LockFile {
    #[serde(default)]
    importers: BTreeMap<String, Importer>,
    #[serde(default)]
    snapshots: BTreeMap<String, Snapshot>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Importer {
    #[serde(default)]
    dependencies: BTreeMap<String, ImporterVersion>,
    #[serde(default)]
    dev_dependencies: BTreeMap<String, ImporterVersion>,
    #[serde(default)]
    optional_dependencies: BTreeMap<String, ImporterVersion>,
}

#[derive(Deserialize)]
struct ImporterVersion {
    version: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
    #[serde(default)]
    optional_dependencies: BTreeMap<String, String>,
}

/// `pnpm-lock.yaml`: what each importer (a package of the workspace, by its folder) and each snapshot (a
/// resolved external package) depend on, as snapshot keys.
pub struct PnpmLock {
    importers: BTreeMap<String, Vec<String>>,
    snapshots: BTreeMap<String, Vec<String>>,
}

impl PnpmLock {
    pub const FILE_NAME: &str = "pnpm-lock.yaml";

    pub fn parse(text: &str) -> Result<Self> {
        let lock: LockFile = serde_yaml_ng::from_str(text)?;
        let importers = lock
            .importers
            .into_iter()
            .map(|(path, importer)| {
                let keys = importer
                    .dependencies
                    .iter()
                    .chain(&importer.dev_dependencies)
                    .chain(&importer.optional_dependencies)
                    .filter_map(|(name, entry)| Self::snapshot_key(name, &entry.version))
                    .collect();

                (path, keys)
            })
            .collect();
        let snapshots = lock
            .snapshots
            .into_iter()
            .map(|(key, snapshot)| {
                let keys = snapshot
                    .dependencies
                    .iter()
                    .chain(&snapshot.optional_dependencies)
                    .filter_map(|(name, version)| Self::snapshot_key(name, version))
                    .collect();

                (key, keys)
            })
            .collect();

        Ok(PnpmLock {
            importers,
            snapshots,
        })
    }

    /// Importers (relative to the root; the lock at `lock`) whose resolved dependencies, through their
    /// snapshots, differ in `after`, a later version of the lock.
    pub fn changes_to(&self, after: &PnpmLock, lock: &Path) -> Vec<LockChange> {
        let workspace = lock.parent().unwrap_or(Path::new(""));

        after
            .importers
            .keys()
            .filter(|importer| {
                *importer != "." && self.closure(importer) != after.closure(importer)
            })
            .map(|importer| LockChange::Folder(workspace.join(importer).normalize()))
            .collect()
    }

    fn closure(&self, importer: &str) -> BTreeSet<String> {
        let mut visits = BTreeSet::new();
        let mut stack = self.importers.get(importer).cloned().unwrap_or_default();

        while let Some(key) = stack.pop() {
            if visits.insert(key.clone()) {
                stack.extend(self.snapshots.get(&key).cloned().unwrap_or_default());
            }
        }

        visits
    }

    /// Snapshot a resolved version names: `name@version`, or the version itself for an alias (`other@1.0.0`); none
    /// for links into the repository.
    fn snapshot_key(name: &str, version: &str) -> Option<String> {
        if version.starts_with("link:") || version.starts_with("file:") {
            return None;
        }

        let alias = version
            .char_indices()
            .skip(1)
            .any(|(_, character)| character == '@')
            && !version.starts_with(|character: char| character.is_ascii_digit());

        Some(if alias {
            version.to_owned()
        } else {
            format!("{name}@{version}")
        })
    }
}
