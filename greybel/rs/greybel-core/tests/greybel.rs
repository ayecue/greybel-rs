//! Snapshot tests for `tests/fixtures/greybel/`; run `INSTA_UPDATE=always cargo test` to update them.

use std::collections::HashMap;

use greybel_core::{Dialect, ParserOptions};

use crate::support::snapshot::{render, render_invalid, render_valid};

#[test]
fn should_parse_every_script_fixture() {
    insta::glob!("fixtures/greybel/scripts/*.src", |path| {
        let source = std::fs::read_to_string(path).expect("a fixture");
        let options = ParserOptions::new(Dialect::Greybel);

        let rendered = render_valid(&source, &options);

        insta::assert_snapshot!(rendered);
    });
}

#[test]
fn should_report_every_error_fixture() {
    insta::glob!("fixtures/greybel/errors/*.src", |path| {
        let source = std::fs::read_to_string(path).expect("a fixture");
        let options = ParserOptions::new(Dialect::Greybel);

        let rendered = render_invalid(&source, &options);

        insta::assert_snapshot!(rendered);
    });
}

/// Tests `#if` resolution with `ENABLED=true`, `DISABLED=false`, and `ZERO=0`.
#[test]
fn should_resolve_if_directives_from_the_environment() {
    insta::glob!("fixtures/greybel/env/*.src", |path| {
        let source = std::fs::read_to_string(path).expect("a fixture");
        let environment: HashMap<String, String> =
            [("ENABLED", "true"), ("DISABLED", "false"), ("ZERO", "0")]
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect();
        let options = ParserOptions {
            filename: "main.src".to_owned(),
            environment,
            ..ParserOptions::new(Dialect::Greybel)
        };

        let rendered = render(&source, &options);

        insta::assert_snapshot!(rendered);
    });
}
