//! greybel-vs's folder and its package manifest, and what the tests read from the libraries' data.

pub mod schema;
pub mod settings;

use std::path::{Path, PathBuf};

use serde_json::Value;

pub fn greybel_vs() -> PathBuf {
    Path::new(env!("APPS_TS")).join("greybel-vs")
}

pub fn greybel_editor_web() -> PathBuf {
    Path::new(env!("APPS_TS")).join("greybel-editor-web")
}

pub fn package() -> Value {
    let path = greybel_vs().join("package.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));

    serde_json::from_str(&text).expect("package.json is JSON")
}

/// The settings `package.json` declares, as its groups of them have them.
pub fn declared_settings() -> Vec<(String, Value)> {
    package()["contributes"]["configuration"]
        .as_array()
        .expect("groups of settings")
        .iter()
        .flat_map(|group| {
            group["properties"]
                .as_object()
                .expect("a group's settings")
                .iter()
                .map(|(key, setting)| (key.clone(), setting.clone()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Writes `generated` to `path` with `UPDATE_TS=1`, then requires `path` to hold it.
pub fn assert_up_to_date(path: &Path, generated: &str) {
    if std::env::var_os("UPDATE_TS").is_some() {
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("the folder is made");
        std::fs::write(path, generated).expect("the file is written");
    }

    let stored = std::fs::read_to_string(path).unwrap_or_default();

    assert!(
        stored == generated,
        "{} is out of date; run `UPDATE_TS=1 cargo test -p greybel-vs-assets`",
        path.display()
    );
}
