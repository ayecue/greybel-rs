//! Tests profiles: named variations of `build` and `run`, merged over the base settings and the profiles they extend.

use greybel_project::{BuildMode, Manifest, Project};

use crate::support::profiles::{PROFILES, project};

#[test]
fn should_merge_a_profile_into_the_settings_it_changes() {
    let manifest = project(PROFILES)
        .and_then(|project| {
            project
                .with_profile(Some("release"))
                .map_err(|error| error.to_string())
        })
        .expect("the release profile")
        .manifest;

    assert_eq!(manifest.build.mode, BuildMode::Uglify);
    assert_eq!(manifest.build.excluded_namespaces, ["c"]);
    assert!(!manifest.build.installer.active);
    assert_eq!(manifest.build.installer.max_chars, 1000);
    assert_eq!(manifest.run.seed, "base");
}

#[test]
fn should_apply_the_profiles_a_profile_extends_first() {
    let manifest = project(PROFILES)
        .expect("a project")
        .manifest
        .with_profile("ingame")
        .expect("the ingame profile");

    assert_eq!(manifest.build.mode, BuildMode::Uglify);
    assert!(manifest.build.obfuscation);
    assert!(!manifest.build.installer.active);
}

#[test]
fn should_merge_the_values_of_maps_key_by_key() {
    let manifest = project(PROFILES)
        .expect("a project")
        .manifest
        .with_profile("debug")
        .expect("the debug profile");

    assert_eq!(manifest.run.seed, "debug");
    assert_eq!(
        manifest.run.variables.values,
        [
            ("A".to_owned(), "1".to_owned()),
            ("B".to_owned(), "2".to_owned())
        ]
        .into()
    );
}

#[test]
fn should_keep_the_base_settings_without_a_profile() {
    let base = project(PROFILES).expect("a project");

    assert_eq!(base.clone().with_profile(None).expect("no profile"), base);
}

#[test]
fn should_refuse_a_profile_the_project_doesnt_have() {
    let error = project(PROFILES)
        .expect("a project")
        .with_profile(Some("staging"))
        .expect_err("no staging profile");

    assert_eq!(
        error.to_string(),
        "file:///work/greybel.json: the project has no profile `staging`"
    );
    assert_eq!(
        Project::default()
            .with_profile(Some("release"))
            .expect_err("no profiles")
            .to_string(),
        "the defaults (no greybel.json): the project has no profile `release`"
    );
}

#[test]
fn should_refuse_profiles_extending_each_other_when_read() {
    let error = project(r#"{ "profiles": { "a": { "extends": "b" }, "b": { "extends": "a" } } }"#)
        .expect_err("a cycle");

    assert_eq!(
        error,
        "file:///work/greybel.json: the profiles extend each other in a cycle: a → b → a"
    );
    assert_eq!(
        project(r#"{ "profiles": { "a": { "extends": "missing" } } }"#)
            .expect_err("an unknown profile"),
        "file:///work/greybel.json: the project has no profile `missing`"
    );
}

#[test]
fn should_refuse_a_profile_with_a_setting_build_doesnt_have_when_read() {
    let error = project(r#"{ "profiles": { "fast": { "build": { "speed": 1 } } } }"#)
        .expect_err("an unknown setting");

    assert!(
        error.starts_with("file:///work/greybel.json: the profile `fast`: unknown field `speed`"),
        "{error}"
    );
    assert!(
        project(r#"{ "profiles": { "fast": { "format": {} } } }"#)
            .expect_err("only build and run")
            .contains("unknown field `format`")
    );
}

#[test]
fn should_describe_profiles_in_the_schema() {
    let schema = Manifest::schema();
    let profile = &schema["$defs"]["Profile"]["properties"];

    assert_eq!(
        schema["properties"]["profiles"]["additionalProperties"]["$ref"],
        "#/$defs/Profile"
    );
    assert_eq!(profile["build"]["$ref"], "#/$defs/BuildSettings");
    assert_eq!(profile["run"]["$ref"], "#/$defs/RunSettings");
    assert_eq!(
        profile["extends"]["type"],
        serde_json::json!(["string", "null"])
    );
}
