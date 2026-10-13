//! greybel.json's JSON Schema, which greybel-vs publishes in `schemas/greybel.schema.json` and syncs its settings
//! by, and the settings it names: each one greybel-vs has, with greybel.json's default, for a workspace folder.

use greybel_project::Manifest;

use crate::support::schema::{vscode_settings, vscode_value};
use crate::support::{assert_up_to_date, declared_settings, greybel_vs};

#[test]
fn should_publish_the_schema_of_greybel_json() {
    let schema = format!(
        "{}\n",
        serde_json::to_string_pretty(&Manifest::schema()).expect("a schema is JSON")
    );

    assert_up_to_date(&greybel_vs().join("schemas/greybel.schema.json"), &schema);
}

#[test]
fn should_name_settings_with_the_defaults_of_greybel_json() {
    let schema = Manifest::schema();
    let defaults = serde_json::to_value(Manifest::default()).expect("the defaults are JSON");
    let declared = declared_settings();
    let mut named = 0;

    for (setting, format, default) in vscode_settings(&schema, &schema, &defaults) {
        let (_, declaration) = declared
            .iter()
            .find(|(key, _)| *key == setting)
            .unwrap_or_else(|| panic!("greybel-vs has no setting {setting}"));

        assert_eq!(
            declaration["default"],
            vscode_value(&default, format.as_deref()),
            "{setting}"
        );
        assert_eq!(
            declaration["scope"], "resource",
            "{setting} is a workspace folder's"
        );

        named += 1;
    }

    assert!(named > 20, "{named} settings");
}
