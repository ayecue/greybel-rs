use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Bundle;

/// Stores the source game version and lowercase SHA-256 hash of each decompressed file in `fingerprint.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    pub game: String,
    pub files: BTreeMap<String, String>,
}

impl Fingerprint {
    /// Deserializes fingerprint JSON.
    ///
    /// # Errors
    ///
    /// Returns an error when `json` is malformed or violates the fingerprint schema.
    pub fn read(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serializes the fingerprint using two-space indentation and a trailing newline.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut json = serde_json::to_string_pretty(self).expect("a fingerprint serializes");

        json.push('\n');

        json
    }
}

impl Bundle {
    /// Computes every bundle file's SHA-256 fingerprint and records game version `game`.
    #[must_use]
    pub fn fingerprint(&self, game: &str) -> Fingerprint {
        let files = self
            .names()
            .map(|name| {
                let digest = Sha256::digest(self.file(name).unwrap_or_default());
                let hex = digest.iter().fold(String::new(), |mut hex, byte| {
                    let _ = write!(hex, "{byte:02x}");

                    hex
                });

                (name.to_owned(), hex)
            })
            .collect();

        Fingerprint {
            game: game.to_owned(),
            files,
        }
    }
}
