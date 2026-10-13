//! Compares `CompareTo` and `EndsWith` against Grey Hack's runtime in the invariant culture, recorded by `make fixtures` in `mono/dotnet/`.

use std::cmp::Ordering;

use mono_string::collation::{compare_units, ends_with_units};
use support::{check_of, load_collation_fixtures, units};

use crate::support;

#[test]
fn should_compare_like_the_game() {
    let fixtures = load_collation_fixtures();
    let texts: Vec<Vec<u16>> = fixtures["texts"]
        .as_array()
        .expect("texts")
        .iter()
        .map(units)
        .collect();
    let expected = fixtures["compare"].as_str().expect("results").as_bytes();
    let outcomes: Vec<Ordering> = texts
        .iter()
        .flat_map(|a| texts.iter().map(|b| compare_units(a, b)))
        .collect();
    let mut differences = Vec::new();

    for (i, a) in texts.iter().enumerate() {
        for (j, b) in texts.iter().enumerate() {
            let game = match expected[i * texts.len() + j] {
                b'-' => Ordering::Less,
                b'0' => Ordering::Equal,
                _ => Ordering::Greater,
            };
            let ours = outcomes[i * texts.len() + j];

            if ours != game {
                differences.push(format!(
                    "{:?}.CompareTo({:?}): game {game:?}, ours {ours:?}",
                    String::from_utf16_lossy(a),
                    String::from_utf16_lossy(b)
                ));
            }
        }
    }

    check_of(&differences, expected.len());
}

#[test]
fn should_end_with_like_the_game() {
    let fixtures = load_collation_fixtures();
    let cases: Vec<(Vec<u16>, Vec<u16>, bool)> = fixtures["ends_with"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| {
            (
                units(&row[0]),
                units(&row[1]),
                row[2].as_bool().expect("a result"),
            )
        })
        .collect();
    let outcomes: Vec<bool> = cases
        .iter()
        .map(|(text, suffix, _)| ends_with_units(text, suffix))
        .collect();
    let differences: Vec<String> = cases
        .iter()
        .zip(&outcomes)
        .filter(|((_, _, game), ours)| *ours != game)
        .map(|((text, suffix, game), ours)| {
            format!(
                "{:?}.EndsWith({:?}): game {game}, ours {ours}",
                String::from_utf16_lossy(text),
                String::from_utf16_lossy(suffix)
            )
        })
        .collect();

    check_of(&differences, cases.len());
}
