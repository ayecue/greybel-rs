#![allow(dead_code)]

use serde::Deserialize;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

pub fn load<T: for<'de> Deserialize<'de>>(name: &str) -> T {
    let text = std::fs::read_to_string(format!("{FIXTURES}/{name}")).expect("a fixture file");

    serde_json::from_str(&text).expect("a valid fixture")
}

pub fn double(bits: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(bits, 16).expect("hex bits"))
}

pub fn float(bits: &str) -> f32 {
    f32::from_bits(u32::from_str_radix(bits, 16).expect("hex bits"))
}

/// Same bits, or both NaN.
pub fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

/// Same bits or both NaN when both are there, or both missing.
pub fn same_or_both_missing(actual: Option<f64>, expected: Option<f64>) -> bool {
    match (actual, expected) {
        (Some(a), Some(b)) => same(a, b),
        (None, None) => true,
        _ => false,
    }
}

pub fn check(differences: &[String]) {
    assert!(
        differences.is_empty(),
        "{} differences from the game:\n{}",
        differences.len(),
        differences
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[derive(Deserialize)]
pub struct FormatFixtures {
    pub double_formats: Vec<String>,
    pub doubles: Vec<Vec<Option<String>>>,
    pub float_formats: Vec<String>,
    pub floats: Vec<Vec<Option<String>>>,
}

#[derive(Deserialize)]
pub struct ParseFixtures {
    pub styles: Vec<u32>,
    pub texts: Vec<Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
pub struct MathFixtures {
    pub digits: Vec<i32>,
    pub values: Vec<Vec<serde_json::Value>>,
}

/// One value printed with one format: its bits, the format's index and
/// text, and what the game printed.
pub struct FormatCase<'f> {
    pub bits: &'f str,
    pub format: usize,
    pub text: &'f str,
    pub expected: &'f Option<String>,
}

/// Every value of `rows` with every one of `formats`.
pub fn format_cases<'f>(
    formats: &'f [String],
    rows: &'f [Vec<Option<String>>],
) -> Vec<FormatCase<'f>> {
    rows.iter()
        .flat_map(|row| {
            let bits = row[0].as_deref().expect("bits");

            formats
                .iter()
                .enumerate()
                .zip(&row[1..])
                .map(move |((format, text), expected)| FormatCase {
                    bits,
                    format,
                    text,
                    expected,
                })
        })
        .collect()
}

/// One text parsed with one style, and the game's double and integer.
pub struct ParseCase<'f> {
    pub text: &'f str,
    pub styles: mono_number::NumberStyles,
    pub number: Option<String>,
    pub integer: Option<i32>,
}

/// Every text of the parse fixtures with every style.
pub fn parse_cases(fixtures: &ParseFixtures) -> Vec<ParseCase<'_>> {
    fixtures
        .texts
        .iter()
        .flat_map(|row| {
            let text = row[0].as_str().expect("a text");

            fixtures
                .styles
                .iter()
                .zip(&row[1..])
                .map(move |(&style, result)| {
                    let styles = mono_number::NumberStyles::from_bits(style).expect("known styles");
                    let (number, integer): (Option<String>, Option<i32>) =
                        serde_json::from_value(result.clone()).expect("a result pair");

                    ParseCase {
                        text,
                        styles,
                        number,
                        integer,
                    }
                })
        })
        .collect()
}
