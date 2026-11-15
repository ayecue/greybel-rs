//! Scanning libraries for weaknesses (`MetaExploitIntrinsics.scan`).

use std::cmp::Ordering;

use greyhack_behaviour::compare_versions;
use greyhack_world_core::{ComputerId, Result};

use crate::{Library, Metaxploit, Weakness};

/// Weak addresses a `scan` lists (`MetaExploitIntrinsics.scan`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryScan {
    /// Addresses with a weakness the metaxploit version can see.
    pub addresses: Vec<String>,
    /// Whether a zone that may contain weaknesses could not be scanned.
    pub hidden_zones: bool,
}

/// Weaknesses a `scan_address` finds (`MetaExploitIntrinsics.scan_address`).
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneScan {
    /// Whether the library has a zone at the address.
    pub zone_exists: bool,
    /// Weaknesses the metaxploit version can see.
    pub weaknesses: Vec<Weakness>,
}

/// Methods of [`Library`] for exploits metaxploit scan.
impl Library {
    pub fn has_weaknesses_at(&self, address: &str) -> bool {
        self.zones
            .iter()
            .find(|zone| zone.address == address)
            .is_some_and(|zone| !zone.weaknesses.is_empty())
    }

    pub fn scan_zone(&self, version: &str, address: &str) -> ZoneScan {
        let zone = self.zones.iter().find(|zone| zone.address == address);

        ZoneScan {
            zone_exists: zone.is_some(),
            weaknesses: zone
                .into_iter()
                .flat_map(|zone| &zone.weaknesses)
                .filter(|weakness| {
                    compare_versions(version, &weakness.metaxploit_version) == Ordering::Greater
                })
                .cloned()
                .collect(),
        }
    }
}

/// Methods of [`World`] for exploits metaxploit scan.
impl Metaxploit {
    pub async fn scan_library(
        &self,
        library: &Library,
        version: &str,
    ) -> Result<Option<LibraryScan>> {
        let Some(latest) = self.generator.latest_full_library(library.kind).await? else {
            return Ok(None);
        };
        let current = compare_versions(&library.version, &latest.version);
        let mut addresses = Vec::new();
        let mut hidden_zones = false;

        for zone in &library.zones {
            if zone.hide && current == Ordering::Equal {
                continue;
            }

            for weakness in &zone.weaknesses {
                if compare_versions(version, &weakness.metaxploit_version) == Ordering::Greater {
                    addresses.push(zone.address.clone());
                    break;
                }

                if !zone.hide || current == Ordering::Less {
                    hidden_zones = true;
                }
            }
        }

        Ok(Some(LibraryScan {
            addresses,
            hidden_zones,
        }))
    }

    pub async fn library_file_path(
        &self,
        computer: ComputerId,
        path: &str,
    ) -> Result<Option<String>> {
        let Some(node) = self.files.lookup.file(computer, path, true).await? else {
            return Ok(None);
        };

        Ok(self
            .generator
            .full_library_of(&node)
            .await?
            .is_some()
            .then_some(node.path))
    }
}
