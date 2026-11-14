//! JSON service configurations in `/server/conf`, using Grey Hack field names.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Default SSH encode-script path.
pub const DEFAULT_ENCODE_PATH: &str = "/server/encode.src";
/// Default SSH decode-program path.
pub const DEFAULT_DECODE_PATH: &str = "/server/decode.bin";
/// Default repository package directory.
pub const DEFAULT_REPOSITORY_PATH: &str = "/server/aptfiles";

/// SSH configuration (`sshd.conf`) controlling login encryption with encode and decode scripts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SshdConfig {
    pub encryption_enabled: bool,
    pub message_encrypted_conn: bool,
    pub path_enc: String,
    pub path_dec: String,
}

impl Default for SshdConfig {
    fn default() -> Self {
        Self {
            encryption_enabled: false,
            message_encrypted_conn: true,
            path_enc: DEFAULT_ENCODE_PATH.to_owned(),
            path_dec: DEFAULT_DECODE_PATH.to_owned(),
        }
    }
}

impl SshdConfig {
    /// Parses configuration only when encryption is enabled. Invalid JSON and disabled encryption
    /// return `None` and skip encryption for the connection.
    #[must_use]
    pub fn parse(content: &str) -> Option<Self> {
        let config: Self = serde_json::from_str(content).ok()?;

        config.encryption_enabled.then_some(config)
    }
}

/// HTTP configuration describing shop software.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpConfig {
    #[serde(rename = "softwareDescription")]
    pub software: IndexMap<String, SoftwareInfo>,
}

/// Program sold by a shop (`Service_httpd.SoftwareInfo`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SoftwareInfo {
    pub description: String,
    pub price: u32,
}

/// Repository configuration with package directory and package descriptions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RepositoryConfig {
    pub path_repo: String,
    pub repo_sources: IndexMap<String, String>,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            path_repo: DEFAULT_REPOSITORY_PATH.to_owned(),
            repo_sources: IndexMap::new(),
        }
    }
}

/// Pretty-printed JSON configuration-file contents.
pub(crate) fn config_text(config: &impl Serialize) -> String {
    serde_json::to_string_pretty(config).expect("a service configuration serializes")
}
