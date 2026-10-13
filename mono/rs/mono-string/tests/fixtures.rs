//! Compares against what Grey Hack's own runtime returned, recorded by `make fixtures` in `mono/dotnet/`.

use std::cmp::Ordering;

use mono_string::{compare_ordinal, hash_code, to_lower, to_upper, utf16};
use support::{check, load_string_fixtures, number, position, rows, text};

use crate::support;

#[test]
fn should_measure_lengths_like_the_game() {
    let fixtures = load_string_fixtures();
    let rows = rows(&fixtures, "texts");
    let lengths: Vec<i64> = rows
        .iter()
        .map(|row| i64::try_from(utf16::len(text(&row[0]))).expect("small"))
        .collect();
    let differences: Vec<String> = rows
        .iter()
        .zip(&lengths)
        .filter(|(row, len)| **len != number(&row[1]))
        .map(|(row, len)| format!("{:?}.Length: game {}, ours {len}", text(&row[0]), row[1]))
        .collect();

    check(&differences);
}

#[test]
fn should_hash_like_the_game() {
    let fixtures = load_string_fixtures();
    let rows = rows(&fixtures, "texts");
    let hashes: Vec<i64> = rows
        .iter()
        .map(|row| i64::from(hash_code(text(&row[0]))))
        .collect();
    let differences: Vec<String> = rows
        .iter()
        .zip(&hashes)
        .filter(|(row, hash)| **hash != number(&row[2]))
        .map(|(row, hash)| {
            format!(
                "{:?}.GetHashCode(): game {}, ours {hash}",
                text(&row[0]),
                row[2]
            )
        })
        .collect();

    check(&differences);
}

#[test]
fn should_upper_case_like_the_game() {
    let fixtures = load_string_fixtures();
    let rows = rows(&fixtures, "texts");
    let uppers: Vec<_> = rows.iter().map(|row| to_upper(text(&row[0]))).collect();
    let differences: Vec<String> = rows
        .iter()
        .zip(&uppers)
        .filter(|(row, upper)| **upper != text(&row[3]))
        .map(|(row, upper)| {
            format!(
                "{:?}.ToUpper(): game {:?}, ours {upper:?}",
                text(&row[0]),
                row[3]
            )
        })
        .collect();

    check(&differences);
}

#[test]
fn should_lower_case_like_the_game() {
    let fixtures = load_string_fixtures();
    let rows = rows(&fixtures, "texts");
    let lowers: Vec<_> = rows.iter().map(|row| to_lower(text(&row[0]))).collect();
    let differences: Vec<String> = rows
        .iter()
        .zip(&lowers)
        .filter(|(row, lower)| **lower != text(&row[4]))
        .map(|(row, lower)| {
            format!(
                "{:?}.ToLower(): game {:?}, ours {lower:?}",
                text(&row[0]),
                row[4]
            )
        })
        .collect();

    check(&differences);
}

#[test]
fn should_compare_like_the_game() {
    let fixtures = load_string_fixtures();
    let rows = rows(&fixtures, "compare");
    let outcomes: Vec<Ordering> = rows
        .iter()
        .map(|row| compare_ordinal(text(&row[0]), text(&row[1])))
        .collect();
    let mut differences = Vec::new();

    for (row, &actual) in rows.iter().zip(&outcomes) {
        let (a, b) = (text(&row[0]), text(&row[1]));
        let expected = number(&row[2]).cmp(&0);

        if actual != expected {
            differences.push(format!(
                "Compare({a:?}, {b:?}): game {expected:?}, ours {actual:?}"
            ));
        }

        if actual != Ordering::Equal && a.cmp(b) != actual {
            assert!(a.chars().chain(b.chars()).any(|c| c >= '\u{E000}'));
        }
    }

    check(&differences);
}

#[test]
fn should_slice_like_the_game() {
    let fixtures = load_string_fixtures();
    let cases: Vec<(&str, usize, usize, String)> = rows(&fixtures, "substring")
        .iter()
        .map(|row| {
            let start = usize::try_from(number(&row[1])).expect("a position");
            let len = usize::try_from(number(&row[2])).expect("a length");
            let units: Vec<u16> = row[3]
                .as_array()
                .expect("units")
                .iter()
                .map(|unit| u16::try_from(number(unit)).expect("a unit"))
                .collect();

            (text(&row[0]), start, len, String::from_utf16_lossy(&units))
        })
        .collect();
    let outcomes: Vec<_> = cases
        .iter()
        .map(|(value, start, len, _)| utf16::substring(value, *start..*start + *len))
        .collect();
    let differences: Vec<String> = cases
        .iter()
        .zip(&outcomes)
        .filter(|((_, _, _, expected), actual)| actual.as_ref() != expected.as_str())
        .map(|((value, start, len, expected), actual)| {
            format!("{value:?}.Substring({start}, {len}): game {expected:?}, ours {actual:?}")
        })
        .collect();

    check(&differences);
}

#[test]
fn should_find_the_first_index_like_the_game() {
    let fixtures = load_string_fixtures();
    let rows = rows(&fixtures, "search");
    let firsts: Vec<i64> = rows
        .iter()
        .map(|row| position(utf16::index_of(text(&row[0]), text(&row[1]))))
        .collect();
    let differences: Vec<String> = rows
        .iter()
        .zip(&firsts)
        .filter(|(row, first)| **first != number(&row[2]))
        .map(|(row, first)| {
            format!(
                "{:?}.IndexOf({:?}): game {}, ours {first}",
                text(&row[0]),
                text(&row[1]),
                row[2]
            )
        })
        .collect();

    check(&differences);
}

#[test]
fn should_find_the_last_index_like_the_game() {
    let fixtures = load_string_fixtures();
    let rows = rows(&fixtures, "search");
    let lasts: Vec<i64> = rows
        .iter()
        .map(|row| position(utf16::last_index_of(text(&row[0]), text(&row[1]))))
        .collect();
    let differences: Vec<String> = rows
        .iter()
        .zip(&lasts)
        .filter(|(row, last)| **last != number(&row[3]))
        .map(|(row, last)| {
            format!(
                "{:?}.LastIndexOf({:?}): game {}, ours {last}",
                text(&row[0]),
                text(&row[1]),
                row[3]
            )
        })
        .collect();

    check(&differences);
}
