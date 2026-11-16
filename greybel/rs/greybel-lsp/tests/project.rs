//! Tests `custom/project`: the project of a document, read through the client.

use greybel_lsp::{Project, ProjectResult};
use serde_json::json;

use crate::support::project::project;
use crate::support::{Client, uri};

#[test]
fn should_read_the_nearest_greybel_json_and_its_variables() {
    let mut client = Client::started(&[
        ("greybel.json", r#"{ "strictMode": false }"#),
        (
            "game/greybel.json",
            r#"{ "strictMode": true, "build": { "variables": { "files": ["build.env"], "values": { "B": "2" } } } }"#,
        ),
        ("game/build.env", "A=1\nB=1"),
        ("game/.env", "RUN=yes"),
        ("game/src/main.src", "print 1"),
    ]);

    let result = project(&mut client, "game/src/main.src", &json!(null));

    assert_eq!(result.root.as_deref(), Some(uri("game").as_str()));
    assert!(result.manifest.strict_mode);
    assert_eq!(
        result.build_variables,
        [("A".into(), "1".into()), ("B".into(), "2".into())].into()
    );
    assert_eq!(result.run_variables, [("RUN".into(), "yes".into())].into());
    assert!(
        client
            .requests
            .iter()
            .any(|method| method == "custom/read-file")
    );
}

#[test]
fn should_take_the_fallback_without_greybel_json_with_files_in_the_workspace_folder() {
    let mut client = Client::started(&[("src/main.src", "print 1"), (".env", "A=1")]);

    let result = project(
        &mut client,
        "src/main.src",
        &json!({ "extensions": ["gs"], "run": { "seed": "other" } }),
    );

    assert_eq!(result.root, None);
    assert_eq!(result.manifest.extensions, ["gs"]);
    assert_eq!(result.manifest.run.seed, "other");
    assert_eq!(result.run_variables, [("A".into(), "1".into())].into());
}

#[test]
fn should_refuse_a_greybel_json_it_cant_read() {
    let mut client =
        Client::started(&[("greybel.json", r#"{ "strict": true }"#), ("main.src", "")]);

    let error = client
        .request_value("custom/project", json!({ "uri": uri("main.src") }))
        .expect_err("an unknown setting");

    assert!(
        error.message.contains("unknown field `strict`"),
        "{error:?}"
    );
}

#[test]
fn should_apply_the_requested_profile_and_read_its_variables() {
    let mut client = Client::started(&[
        (
            "greybel.json",
            r#"{ "profiles": { "debug": { "run": { "seed": "debug", "variables": { "files": ["debug.env"] } } } } }"#,
        ),
        ("debug.env", "DEBUG=1"),
        ("main.src", ""),
    ]);

    let result: ProjectResult =
        client.request::<Project>(json!({ "uri": uri("main.src"), "profile": "debug" }));

    assert_eq!(result.manifest.run.seed, "debug");
    assert_eq!(result.run_variables, [("DEBUG".into(), "1".into())].into());
    assert!(result.manifest.profiles.contains_key("debug"));

    let error = client
        .request_value(
            "custom/project",
            json!({ "uri": uri("main.src"), "profile": "staging" }),
        )
        .expect_err("an unknown profile");

    assert!(
        error
            .message
            .contains("the project has no profile \"staging\""),
        "{error:?}"
    );
}
