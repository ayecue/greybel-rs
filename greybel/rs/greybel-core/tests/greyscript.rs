//! Snapshot tests for `tests/fixtures/greyscript/`; run `INSTA_UPDATE=always cargo test` to update them.

use greybel_core::{Dialect, ParserOptions};

use crate::support::snapshot::{render_invalid, render_valid};

#[test]
fn should_parse_every_script_fixture() {
    insta::glob!("fixtures/greyscript/scripts/*.src", |path| {
        let source = std::fs::read_to_string(path).expect("a fixture");
        let options = ParserOptions::new(Dialect::GreyScript);

        let rendered = render_valid(&source, &options);

        insta::assert_snapshot!(rendered);
    });
}

#[test]
fn should_report_every_error_fixture() {
    insta::glob!("fixtures/greyscript/errors/*.src", |path| {
        let source = std::fs::read_to_string(path).expect("a fixture");
        let options = ParserOptions::new(Dialect::GreyScript);

        let rendered = render_invalid(&source, &options);

        insta::assert_snapshot!(rendered);
    });
}
