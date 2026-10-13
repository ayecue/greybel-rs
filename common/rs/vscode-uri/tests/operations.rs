//! Snapshot tests for all URI and path operations over `tests/fixtures/inputs.json`. Baselines were recorded from
//! vscode-uri 3.2.0 and Node's `path.posix`; snapshot differences indicate behavioral differences. File paths use
//! POSIX semantics.

#![cfg(not(windows))]

use support::operations::{
    ManyPaths, Names, OnePath, base, cases, change, inputs, list, outcome, strings, text,
};
use vscode_uri::{PosixPath, Uri, UriComponentContainer};

use crate::support;

#[test]
fn should_parse() {
    let inputs = inputs();

    let cases = cases(list(&inputs, "parse"), |input| {
        outcome(Uri::parse(text(input)))
    });

    insta::assert_yaml_snapshot!(cases);
}

#[test]
fn should_parse_strictly() {
    let inputs = inputs();

    let cases = cases(list(&inputs, "parse"), |input| {
        outcome(Uri::parse_strict(text(input)))
    });

    insta::assert_yaml_snapshot!(cases);
}

#[test]
fn should_build_file_uris() {
    let inputs = inputs();

    let cases = cases(list(&inputs, "file"), |input| {
        outcome(Ok(Uri::file(text(input))))
    });

    insta::assert_yaml_snapshot!(cases);
}

#[test]
fn should_build_from_components() {
    let inputs = inputs();

    let cases = cases(list(&inputs, "from"), |input| {
        let field = |name: &str| input.get(name).map(text).unwrap_or_default().to_owned();

        outcome(Uri::from_components(UriComponentContainer {
            scheme: field("scheme"),
            authority: field("authority"),
            path: field("path"),
            query: field("query"),
            fragment: field("fragment"),
        }))
    });

    insta::assert_yaml_snapshot!(cases);
}

#[test]
fn should_apply_changes() {
    let inputs = inputs();

    let cases = cases(list(&inputs, "with"), |input| {
        outcome(base(input).with(change(&input["change"])))
    });

    insta::assert_yaml_snapshot!(cases);
}

#[test]
fn should_join_paths() {
    let inputs = inputs();

    let cases = cases(list(&inputs, "joinPath"), |input| {
        outcome(base(input).join_path(&strings(&input["paths"])))
    });

    insta::assert_yaml_snapshot!(cases);
}

#[test]
fn should_resolve_paths() {
    let inputs = inputs();

    let cases = cases(list(&inputs, "resolvePath"), |input| {
        outcome(base(input).resolve_path(&strings(&input["paths"])))
    });

    insta::assert_yaml_snapshot!(cases);
}

#[test]
fn should_take_the_dirname() {
    let inputs = inputs();

    let cases = cases(list(&inputs, "dirname"), |input| {
        outcome(Uri::parse(text(input)).expect("a base").dirname())
    });

    insta::assert_yaml_snapshot!(cases);
}

#[test]
fn should_take_basename_and_extname() {
    let inputs = inputs();

    let cases = cases(list(&inputs, "names"), |input| {
        let uri = Uri::parse(text(input)).expect("a base");

        Names {
            basename: uri.basename().to_owned(),
            extname: uri.extname().to_owned(),
        }
    });

    insta::assert_yaml_snapshot!(cases);
}

#[test]
fn should_apply_posix_single_path_functions() {
    let inputs = inputs();
    let posix = &inputs["posix"];
    let functions: [(&str, OnePath); 4] = [
        ("normalize", PosixPath::normalize),
        ("dirname", |path| PosixPath::dirname(path).to_owned()),
        ("basename", |path| PosixPath::basename(path).to_owned()),
        ("extname", |path| PosixPath::extname(path).to_owned()),
    ];

    let snapshots = functions.map(|(name, function)| {
        (
            name,
            cases(list(posix, name), |input| function(text(input))),
        )
    });

    for (name, cases) in snapshots {
        insta::assert_yaml_snapshot!(format!("posix_{name}"), cases);
    }
}

#[test]
fn should_apply_posix_multi_path_functions() {
    let inputs = inputs();
    let posix = &inputs["posix"];
    let functions: [(&str, ManyPaths); 2] =
        [("join", PosixPath::join), ("resolve", PosixPath::resolve)];

    let snapshots = functions.map(|(name, function)| {
        (
            name,
            cases(list(posix, name), |input| function(&strings(input))),
        )
    });

    for (name, cases) in snapshots {
        insta::assert_yaml_snapshot!(format!("posix_{name}"), cases);
    }
}
