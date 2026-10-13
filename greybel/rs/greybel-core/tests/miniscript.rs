//! Snapshot tests for `tests/fixtures/miniscript/`; run `INSTA_UPDATE=always cargo test` to update them.

use greybel_core::{ParserOptions, Version};

use crate::support::snapshot::{render, render_invalid, render_valid};
use crate::support::{miniscript_options, render_rule_case, rule_cases};

#[test]
fn should_parse_every_script_fixture() {
    insta::glob!("fixtures/miniscript/scripts/*.ms", |path| {
        let source = std::fs::read_to_string(path).expect("a fixture");
        let options = miniscript_options(Version::Latest);

        let rendered = render_valid(&source, &options);

        insta::assert_snapshot!(rendered);
    });
}

#[test]
fn should_report_every_error_fixture() {
    insta::glob!("fixtures/miniscript/errors/*.ms", |path| {
        let source = std::fs::read_to_string(path).expect("a fixture");
        let options = miniscript_options(Version::Latest);

        let rendered = render_invalid(&source, &options);

        insta::assert_snapshot!(rendered);
    });
}

#[test]
fn should_parse_version_fixtures_per_version() {
    insta::glob!("fixtures/miniscript/versions/*.ms", |path| {
        let source = std::fs::read_to_string(path).expect("a fixture");
        let v1_6 = miniscript_options(Version::V1_6);
        let latest = miniscript_options(Version::Latest);

        let rendered = format!(
            "=== 1.6\n{}=== latest\n{}",
            render(&source, &v1_6),
            render(&source, &latest)
        );

        insta::assert_snapshot!(rendered);
    });
}

#[test]
fn should_keep_comments_and_empty_lines_when_preserving() {
    insta::glob!("fixtures/miniscript/preserve/*.ms", |path| {
        let source = std::fs::read_to_string(path).expect("a fixture");
        let options = ParserOptions {
            preserve: true,
            ..miniscript_options(Version::Latest)
        };

        let rendered = render_valid(&source, &options);

        insta::assert_snapshot!(rendered);
    });
}

/// Parses each `---`-separated program in rule fixtures with MiniScript 1.6 and `Latest`.
#[test]
fn should_parse_every_rule_case_in_both_versions() {
    insta::glob!("fixtures/miniscript/rules/*.ms", |path| {
        let source = std::fs::read_to_string(path).expect("a fixture");
        let cases = rule_cases(&source);

        let rendered: String = cases.iter().map(|case| render_rule_case(case)).collect();

        insta::assert_snapshot!(rendered);
    });
}
