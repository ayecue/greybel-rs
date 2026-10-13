//! Compares against what Grey Hack's own runtime returned, recorded by `make fixtures` in `mono/dotnet/`.

use mono_number::{Format, NumberStyles, hash_code, parse_f64, parse_i32, round, round_to_even};
use support::{
    FormatFixtures, MathFixtures, ParseFixtures, check, double, float, format_cases, load,
    parse_cases, same, same_or_both_missing,
};

use crate::support;

#[test]
fn should_format_doubles_like_the_game() {
    let fixtures: FormatFixtures = load("format.json");
    let formats: Vec<Format> = fixtures
        .double_formats
        .iter()
        .map(|f| Format::new(f))
        .collect();
    let cases = format_cases(&fixtures.double_formats, &fixtures.doubles);
    let outcomes: Vec<Option<String>> = cases
        .iter()
        .map(|case| formats[case.format].f64(double(case.bits)).ok())
        .collect();
    let differences: Vec<String> = cases
        .iter()
        .zip(&outcomes)
        .filter(|(case, actual)| *actual != case.expected)
        .map(|(case, actual)| {
            let (bits, text, expected, value) =
                (case.bits, case.text, case.expected, double(case.bits));

            format!("double {value:e} ({bits}) {text:?}: game {expected:?}, ours {actual:?}")
        })
        .collect();

    check(&differences);
}

#[test]
fn should_format_floats_like_the_game() {
    let fixtures: FormatFixtures = load("format.json");
    let formats: Vec<Format> = fixtures
        .float_formats
        .iter()
        .map(|f| Format::new(f))
        .collect();
    let cases = format_cases(&fixtures.float_formats, &fixtures.floats);
    let outcomes: Vec<Option<String>> = cases
        .iter()
        .map(|case| formats[case.format].f32(float(case.bits)).ok())
        .collect();
    let differences: Vec<String> = cases
        .iter()
        .zip(&outcomes)
        .filter(|(case, actual)| *actual != case.expected)
        .map(|(case, actual)| {
            let (bits, text, expected, value) =
                (case.bits, case.text, case.expected, float(case.bits));

            format!("float {value:e} ({bits}) {text:?}: game {expected:?}, ours {actual:?}")
        })
        .collect();

    check(&differences);
}

#[test]
fn should_parse_doubles_like_the_game() {
    let fixtures: ParseFixtures = load("parse.json");
    let cases: Vec<_> = parse_cases(&fixtures)
        .into_iter()
        .filter(|case| !case.styles.contains(NumberStyles::ALLOW_HEX_SPECIFIER))
        .collect();
    let outcomes: Vec<Option<f64>> = cases
        .iter()
        .map(|case| parse_f64(case.text, case.styles))
        .collect();
    let differences: Vec<String> = cases
        .iter()
        .zip(&outcomes)
        .filter_map(|(case, &actual)| {
            let (text, styles) = (case.text, case.styles);
            let expected = case.number.as_deref().map(double);

            (!same_or_both_missing(actual, expected)).then(|| {
                format!("double.TryParse({text:?}, {styles:?}): game {expected:?}, ours {actual:?}")
            })
        })
        .collect();

    check(&differences);
}

#[test]
fn should_parse_integers_like_the_game() {
    let fixtures: ParseFixtures = load("parse.json");
    let cases = parse_cases(&fixtures);
    let outcomes: Vec<Option<i32>> = cases
        .iter()
        .map(|case| parse_i32(case.text, case.styles))
        .collect();
    let differences: Vec<String> = cases
        .iter()
        .zip(&outcomes)
        .filter(|(case, actual)| **actual != case.integer)
        .map(|(case, actual)| {
            let (text, styles, integer) = (case.text, case.styles, case.integer);

            format!("int.TryParse({text:?}, {styles:?}): game {integer:?}, ours {actual:?}")
        })
        .collect();

    check(&differences);
}

#[test]
fn should_round_to_even_like_the_game() {
    let fixtures: MathFixtures = load("math.json");
    let cases: Vec<(f64, f64)> = fixtures
        .values
        .iter()
        .map(|row| {
            (
                double(row[0].as_str().expect("bits")),
                double(row[1].as_str().expect("bits")),
            )
        })
        .collect();
    let outcomes: Vec<f64> = cases
        .iter()
        .map(|&(value, _)| round_to_even(value))
        .collect();
    let differences: Vec<String> = cases
        .iter()
        .zip(&outcomes)
        .filter(|((_, expected), actual)| !same(**actual, *expected))
        .map(|((value, expected), actual)| {
            format!("Math.Round({value:e}): game {expected:e}, ours {actual:e}")
        })
        .collect();

    check(&differences);
}

#[test]
fn should_hash_doubles_like_the_game() {
    let fixtures: MathFixtures = load("math.json");
    let cases: Vec<(f64, i64)> = fixtures
        .values
        .iter()
        .map(|row| {
            (
                double(row[0].as_str().expect("bits")),
                row[2].as_i64().expect("a hash"),
            )
        })
        .collect();
    let outcomes: Vec<i64> = cases
        .iter()
        .map(|&(value, _)| i64::from(hash_code(value)))
        .collect();
    let differences: Vec<String> = cases
        .iter()
        .zip(&outcomes)
        .filter(|((_, expected), actual)| *actual != expected)
        .map(|((value, expected), actual)| {
            format!("{value:e}.GetHashCode(): game {expected}, ours {actual}")
        })
        .collect();

    check(&differences);
}

#[test]
fn should_round_to_digits_like_the_game() {
    let fixtures: MathFixtures = load("math.json");
    let cases: Vec<(f64, i32, Option<f64>)> = fixtures
        .values
        .iter()
        .flat_map(|row| {
            let value = double(row[0].as_str().expect("bits"));

            fixtures
                .digits
                .iter()
                .zip(&row[3..])
                .map(move |(&digits, expected)| (value, digits, expected.as_str().map(double)))
        })
        .collect();
    let outcomes: Vec<Option<f64>> = cases
        .iter()
        .map(|&(value, digits, _)| round(value, digits))
        .collect();
    let differences: Vec<String> = cases
        .iter()
        .zip(&outcomes)
        .filter(|((_, _, expected), actual)| !same_or_both_missing(**actual, *expected))
        .map(|((value, digits, expected), actual)| {
            format!("Math.Round({value:e}, {digits}): game {expected:?}, ours {actual:?}")
        })
        .collect();

    check(&differences);
}
