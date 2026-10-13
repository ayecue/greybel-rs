//! Tests where a document's project is found, and the files it names.

use std::collections::BTreeMap;

use greybel_documents::Uri;
use greybel_project::{Manifest, Project};

#[test]
fn should_look_for_greybel_json_from_the_documents_folder_up() {
    let locations: Vec<String> = Project::locations(&Uri::file("/work/game/src/main.src"))
        .iter()
        .map(Uri::fs_path)
        .collect();

    assert_eq!(
        locations,
        [
            "/work/game/src/greybel.json",
            "/work/game/greybel.json",
            "/work/greybel.json",
            "/greybel.json"
        ]
    );
}

#[test]
fn should_take_the_nearest_greybel_json() {
    let files = BTreeMap::from([
        ("/work/game/greybel.json", r#"{ "strictMode": true }"#),
        ("/work/greybel.json", r#"{ "extensions": ["gs"] }"#),
    ]);
    let project = Project::discover(&Uri::file("/work/game/src/main.src"), |location| {
        files
            .get(location.fs_path().as_str())
            .map(|text| (*text).to_owned())
    })
    .expect("a project");

    assert_eq!(
        project.root.map(|root| root.fs_path()).as_deref(),
        Some("/work/game")
    );
    assert!(project.manifest.strict_mode);
    assert_eq!(project.manifest.extensions, Manifest::DEFAULT_EXTENSIONS);
}

#[test]
fn should_have_the_defaults_without_greybel_json() {
    let project = Project::discover(&Uri::file("/work/main.src"), |_| None).expect("a project");

    assert_eq!(project, Project::default());
    assert_eq!(project.root, None);
}

#[test]
fn should_report_a_greybel_json_it_cant_read() {
    let error = Project::discover(&Uri::file("/work/main.src"), |_| {
        Some("{ \"strict\": true }".to_owned())
    })
    .expect_err("an unknown setting");

    assert!(
        error
            .to_string()
            .starts_with("file:///work/greybel.json: unknown field `strict`"),
        "{error}"
    );
}

#[test]
fn should_resolve_entries_and_variable_files_in_the_projects_folder() {
    let project = Project::read(
        &Uri::file("/work/greybel.json"),
        r#"{ "entries": ["src/main.src"], "run": { "variables": { "files": ["run.env", "/etc/shared.env"] } } }"#,
    )
    .expect("a project");
    let elsewhere = Uri::file("/elsewhere");

    assert_eq!(
        project
            .entries(&elsewhere)
            .iter()
            .map(Uri::fs_path)
            .collect::<Vec<_>>(),
        ["/work/src/main.src"]
    );
    assert_eq!(
        project
            .variable_files(&elsewhere, &project.manifest.run.variables)
            .iter()
            .map(Uri::fs_path)
            .collect::<Vec<_>>(),
        ["/work/run.env", "/etc/shared.env"]
    );
}

#[test]
fn should_resolve_files_in_the_given_folder_without_greybel_json() {
    let project = Project::default();

    assert_eq!(
        project
            .variable_files(&Uri::file("/work"), &project.manifest.build.variables)
            .iter()
            .map(Uri::fs_path)
            .collect::<Vec<_>>(),
        ["/work/.env"]
    );
}

#[test]
fn should_read_variables_from_files_then_values() {
    let project = Project::read(
        &Uri::file("/work/greybel.json"),
        r#"{ "build": { "variables": { "files": ["a.env", "missing.env", "b.env"], "values": { "B": "value" } } } }"#,
    )
    .expect("a project");
    let files = BTreeMap::from([
        ("/work/a.env", "# a comment\nA=1\nB=from a\n\nEMPTY=\n"),
        ("/work/b.env", "A=2\n"),
    ]);
    let variables = project.read_variables(
        &Uri::file("/elsewhere"),
        &project.manifest.build.variables,
        |file| {
            files
                .get(file.fs_path().as_str())
                .map(|text| (*text).to_owned())
        },
    );

    assert_eq!(
        variables,
        BTreeMap::from([
            ("A".to_owned(), "2".to_owned()),
            ("B".to_owned(), "value".to_owned())
        ])
    );
}
