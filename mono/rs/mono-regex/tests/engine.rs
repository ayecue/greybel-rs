//! The .NET engine, and where the translation differs from it. Needs the
//! `dotnet` feature: `cargo test -p mono-regex --features dotnet`.
#![cfg(feature = "dotnet")]

use std::fmt::Write as _;

use mono_regex::{Match, NativeRegexEngine, RegexEngine, RegexError, RegexOptions};
use support::engine::{CASES, NONE, REPLACEMENTS, describe, dotnet, dotnet_names};

use crate::support;

#[test]
fn should_match_ignoring_case() {
    let engine = dotnet();
    let matched = engine.is_match("Hello", "^h", RegexOptions::IGNORE_CASE);

    assert_eq!(matched, Ok(true));
}

#[test]
fn should_split_around_captures_in_number_order() {
    let engine = dotnet();
    let parts = engine.split("a-=b", "(?<x>-)(=)", NONE);

    assert_eq!(
        parts,
        Ok(vec!["a".into(), "=".into(), "-".into(), "b".into()])
    );
}

#[test]
fn should_replace_with_group_references() {
    let engine = dotnet();
    let replaced = engine.replace("John Smith", r"(\w+) (\w+)", "$2, $1", NONE);

    assert_eq!(replaced, Ok("Smith, John".into()));
}

#[test]
fn should_index_matches_in_utf16_units() {
    let engine = dotnet();
    let found = engine.matches("aé😀b😀", "😀|b", NONE);
    let indices: Vec<usize> = found
        .expect("matches")
        .iter()
        .map(|found| found.index)
        .collect();

    assert_eq!(indices, [2, 4, 5]);
}

#[test]
fn should_report_a_dotnet_parse_error() {
    let engine = dotnet();
    let result = engine.is_match("", "a**", NONE);

    assert_eq!(
        result,
        Err(RegexError::Failure(
            "parsing \"a**\" - Nested quantifier *.".into()
        ))
    );
}

#[test]
fn should_report_a_timeout() {
    let engine = dotnet();
    let input = format!("{}b", "a".repeat(40));
    let result = engine.is_match(&input, "(a|aa)+(?=b)c", NONE);

    assert_eq!(result, Err(RegexError::Timeout));
}

#[test]
fn should_report_the_translation_differences() {
    let dotnet = dotnet();
    let native = NativeRegexEngine::default();
    let mut calls: Vec<(String, String, String)> = Vec::new();

    for &(input, pattern, options) in CASES {
        let call =
            |name: &str| format!("{name}({input:?}, {pattern:?}, {})", dotnet_names(options));

        calls.push((
            call("is_match"),
            describe(&dotnet.is_match(input, pattern, options)),
            describe(&native.is_match(input, pattern, options)),
        ));

        let matches = |engine: &dyn RegexEngine| -> Result<Vec<Match>, RegexError> {
            engine.matches(input, pattern, options)
        };

        calls.push((
            call("matches"),
            describe(&matches(&dotnet)),
            describe(&matches(&native)),
        ));
        calls.push((
            call("split"),
            describe(&dotnet.split(input, pattern, options)),
            describe(&native.split(input, pattern, options)),
        ));
    }

    for &(input, pattern, replacement) in REPLACEMENTS {
        calls.push((
            format!("replace({input:?}, {pattern:?}, {replacement:?})"),
            describe(&dotnet.replace(input, pattern, replacement, NONE)),
            describe(&native.replace(input, pattern, replacement, NONE)),
        ));
    }

    let mut differences = String::new();

    for (call, game, translated) in calls
        .iter()
        .filter(|(_, game, translated)| game != translated)
    {
        let _ = writeln!(
            differences,
            "{call}\n  game:       {game}\n  translated: {translated}"
        );
    }

    insta::assert_snapshot!(differences);
}
