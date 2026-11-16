//! Tests what `greybel.json` reads as.

use std::collections::{HashMap, HashSet};

use greybel_core::SyntaxFeatures;
use greybel_project::{BuildMode, Indentation, Manifest};
use greybel_type_analyzer::{DiagnosticKind, Severity};

#[test]
fn should_read_an_empty_greybel_json_as_the_defaults() {
    let manifest: Manifest = serde_json::from_str("{}").expect("a manifest");

    assert_eq!(manifest, Manifest::default());
    assert_eq!(manifest.extensions, ["src", "gs", "ms"]);
    assert_eq!(manifest.build.ingame_directory, "/root/");
    assert_eq!(manifest.build.installer.max_chars, 160_000);
    assert!(manifest.build.installer.active && manifest.build.auto_compile);
    assert_eq!(manifest.build.variables.files, [".env"]);
    assert_eq!(manifest.run.seed, "test");
    assert_eq!(manifest.run.program_name, "myprogram");
}

#[test]
fn should_keep_the_defaults_a_greybel_json_leaves_out() {
    let manifest: Manifest = serde_json::from_str(
        r#"{
            "$schema": "./greybel.schema.json",
            "format": { "indentation": "Whitespace" },
            "build": { "mode": "Beautify", "installer": { "active": false } }
        }"#,
    )
    .expect("a manifest");

    assert_eq!(manifest.schema.as_deref(), Some("./greybel.schema.json"));
    assert_eq!(manifest.format.indentation, Indentation::Whitespace);
    assert_eq!(manifest.format.indentation_spaces, 2);
    assert_eq!(manifest.build.mode, BuildMode::Beautify);
    assert!(!manifest.build.installer.active);
    assert_eq!(manifest.build.installer.max_chars, 160_000);
}

#[test]
fn should_refuse_a_setting_it_doesnt_have() {
    let error = serde_json::from_str::<Manifest>(r#"{ "build": { "installers": {} } }"#)
        .expect_err("an unknown setting");

    assert!(
        error.to_string().starts_with("unknown field `installers`"),
        "{error}"
    );
}

#[test]
fn should_read_extensions_without_their_dots() {
    let manifest: Manifest =
        serde_json::from_str(r#"{ "extensions": [".src", " gs ", ""] }"#).expect("a manifest");

    assert_eq!(manifest.extensions, ["src", "gs"]);
}

#[test]
fn should_read_an_empty_output_name_as_none() {
    let manifest: Manifest =
        serde_json::from_str(r#"{ "build": { "outputName": "" } }"#).expect("a manifest");

    assert_eq!(manifest.build.output_name, None);
}

#[test]
fn should_turn_diagnostic_levels_into_the_analyzers_options() {
    let manifest: Manifest = serde_json::from_str(
        r#"{
            "diagnostics": {
                "used-before-defined": "off",
                "shadowing": "error",
                "call-signature-mismatch": "warning",
                "no-such-diagnostic": "off"
            }
        }"#,
    )
    .expect("a manifest");

    let options = manifest.diagnostic_options();
    let defaults = Manifest::default().diagnostic_options();

    assert_eq!(
        options.disabled_kinds,
        HashSet::from([DiagnosticKind::UseBeforeDefinition])
    );
    assert_eq!(
        options.severity_overrides,
        HashMap::from([
            (DiagnosticKind::Shadow, Severity::Error),
            (DiagnosticKind::CallSignatureMismatch, Severity::Warning),
        ])
    );
    assert_eq!(
        defaults.disabled_kinds,
        HashSet::from([DiagnosticKind::CallSignatureMismatch])
    );
}

#[test]
fn should_read_syntax_features_turned_off() {
    let manifest: Manifest =
        serde_json::from_str(r#"{ "syntax": { "ternary": false, "classes": true } }"#)
            .expect("a manifest");

    let features = manifest.syntax.features();

    assert!(!features.contains(SyntaxFeatures::TERNARY));
    assert!(features.contains(SyntaxFeatures::CLASSES));
    assert_eq!(
        manifest.syntax.off_names(),
        Some(vec!["ternary".to_owned()])
    );
    assert_eq!(Manifest::default().syntax.off_names(), None);
}
