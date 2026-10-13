//! A `greybel.json` with profiles, read as it is at `/work/greybel.json`.

use greybel_documents::Uri;
use greybel_project::Project;

/// Base settings and three profiles: `release`, `ingame` extending it, and `debug`.
pub const PROFILES: &str = r#"{
    "build": { "obfuscation": false, "excludedNamespaces": ["a", "b"], "installer": { "maxChars": 1000 } },
    "run": { "seed": "base", "variables": { "values": { "A": "1", "B": "1" } } },
    "profiles": {
        "release": {
            "build": { "mode": "Uglify", "excludedNamespaces": ["c"], "installer": { "active": false } }
        },
        "ingame": { "extends": "release", "build": { "obfuscation": true } },
        "debug": { "run": { "seed": "debug", "variables": { "values": { "B": "2" } } } }
    }
}"#;

pub fn project(text: &str) -> Result<Project, String> {
    Project::read(&Uri::file("/work/greybel.json"), text).map_err(|error| error.to_string())
}
