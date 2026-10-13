use std::sync::LazyLock;

use serde::Deserialize;

/// Service configuration from the game's `Servicios` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDefinition {
    pub name: String,
    pub port: u16,
    pub daemon: String,
    pub config_path: String,
    pub executable_path: String,
}

#[derive(Deserialize)]
struct RawService {
    puerto: u16,
    #[serde(rename = "pathExe")]
    path_exe: String,
    #[serde(rename = "pathDb")]
    path_db: String,
    nomdb: String,
    nomexe: String,
}

static ALL: LazyLock<Vec<ServiceDefinition>> = LazyLock::new(|| {
    let raw: std::collections::BTreeMap<String, RawService> =
        serde_json::from_str(crate::bundle::text("config/services.json"))
            .expect("the game's service table is valid JSON");

    raw.into_iter()
        .map(|(name, service)| {
            let executable_path = format!("{}{}", service.path_exe, service.nomexe);

            ServiceDefinition {
                daemon: service.nomexe,
                config_path: format!("{}{}", service.path_db, service.nomdb),
                executable_path,
                name,
                port: service.puerto,
            }
        })
        .collect()
});

impl ServiceDefinition {
    #[must_use]
    pub fn find(name: &str) -> Option<&'static Self> {
        ALL.iter().find(|service| service.name == name)
    }
}

/// Returns SSH encode script source for `/server/encode.src`.
#[must_use]
pub fn ssh_encode() -> &'static str {
    crate::bundle::text("config/encode.src")
}

/// Returns source for the compiled SSH decode script `/server/decode.bin`.
#[must_use]
pub fn ssh_decode() -> &'static str {
    crate::bundle::text("config/decode.bin")
}

/// Returns default HTML for a new HTTP service.
#[must_use]
pub fn website_default() -> &'static str {
    crate::bundle::text("config/default.html")
}

/// Returns game text file `name` (`XmlGlobal.CacheText.GetContentFile`), such as the website for `Router`, a
/// network kind (`Comisaria`), or `cctv`. Returns the default website when no file exists for `name`.
#[must_use]
pub fn content_file(name: &str) -> &'static str {
    crate::bundle::BUNDLE
        .text(&format!("texts/files/{name}.txt"))
        .unwrap_or_else(website_default)
}
