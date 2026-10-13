//! The `greybel` section greybel-vs sends the language server.

use serde_json::{Map, Value};

use super::declared_settings;

/// The nested `greybel` section of the declared settings' defaults.
pub fn section() -> Value {
    let mut section = Value::Object(Map::new());

    for (key, setting) in declared_settings() {
        let Some(default) = setting.get("default") else {
            continue;
        };
        let key = key.strip_prefix("greybel.").expect("a greybel setting");
        let (parents, name) = key.rsplit_once('.').unwrap_or(("", key));
        let mut parent = &mut section;

        for segment in parents.split('.').filter(|segment| !segment.is_empty()) {
            parent = parent
                .as_object_mut()
                .expect("an object")
                .entry(segment)
                .or_insert_with(|| Value::Object(Map::new()));
        }

        parent[name] = default.clone();
    }

    section
}
