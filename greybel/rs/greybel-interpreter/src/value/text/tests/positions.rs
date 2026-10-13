#[path = "support/positions.rs"]
mod support;

use support::{ENCODINGS, Sample, TEXT, find};

use super::*;

#[test]
fn should_find_in_units() {
    let cases = [
        ("aé😀b😀", "b", 0, PositionEncoding::Utf16),
        ("aé😀b😀", "b", 0, PositionEncoding::Utf8),
        ("aé😀b😀", "b", 0, PositionEncoding::Utf32),
        ("aé😀b😀", "😀", 3, PositionEncoding::Utf16),
        ("aé😀b😀", "😀", 3, PositionEncoding::Utf32),
        ("abcabc", "c", 3, PositionEncoding::Utf16),
        ("abc", "x", 0, PositionEncoding::Utf16),
    ];
    let found =
        cases.map(|(text, search_text, from, encoding)| find(text, search_text, from, encoding));

    assert_eq!(found[0], Some(4));
    assert_eq!(found[1], Some(7));
    assert_eq!(found[2], Some(3));
    assert_eq!(found[3], Some(5));
    assert_eq!(found[4], Some(4));
    assert_eq!(found[5], Some(5));
    assert_eq!(found[6], None);
}

#[test]
fn should_count_ascii_in_utf16_code_units() {
    let text = Sample::new("abc", PositionEncoding::Utf16);
    let len = text.len();

    assert_eq!(len, 3);
}

#[test]
fn should_count_utf16_code_units() {
    let text = Sample::new(TEXT, PositionEncoding::Utf16);
    let len = text.len();

    assert_eq!(len, 5);
}

#[test]
fn should_slice_utf16_code_units() {
    let text = Sample::new(TEXT, PositionEncoding::Utf16);
    let sliced = text.slice(1..4);

    assert_eq!(sliced, "é😀");
}

#[test]
fn should_index_utf16_code_units() {
    let text = Sample::new(TEXT, PositionEncoding::Utf16);
    let unit = text.at(4);

    assert_eq!(unit, "b");
}

#[test]
fn should_read_half_a_surrogate_pair_as_a_replacement() {
    let text = Sample::new("😀", PositionEncoding::Utf16);
    let unit = text.at(0);

    assert_eq!(unit, "\u{FFFD}");
}

#[test]
fn should_repeat_utf16_text_a_fraction_of_times() {
    let ab = Sample::new("ab", PositionEncoding::Utf16);
    let repeated = ab.repeat(2.5);

    assert_eq!(repeated.as_deref(), Some("ababa"));
}

#[test]
fn should_not_repeat_text_nan_times() {
    let ab = Sample::new("ab", PositionEncoding::Utf16);
    let repeated = ab.repeat(f64::NAN);

    assert_eq!(repeated, None);
}

#[test]
fn should_count_utf8_bytes() {
    let text = Sample::new(TEXT, PositionEncoding::Utf8);
    let len = text.len();

    assert_eq!(len, 8);
}

#[test]
fn should_slice_utf8_bytes() {
    let text = Sample::new(TEXT, PositionEncoding::Utf8);
    let sliced = text.slice(1..3);

    assert_eq!(sliced, "é");
}

#[test]
fn should_index_utf8_bytes() {
    let text = Sample::new(TEXT, PositionEncoding::Utf8);
    let unit = text.at(7);

    assert_eq!(unit, "b");
}

#[test]
fn should_read_part_of_a_utf8_sequence_as_a_replacement() {
    let text = Sample::new(TEXT, PositionEncoding::Utf8);
    let unit = text.at(1);

    assert_eq!(unit, "\u{FFFD}");
}

#[test]
fn should_repeat_utf8_text_a_fraction_of_times() {
    let text = Sample::new("é", PositionEncoding::Utf8);
    let repeated = text.repeat(1.5);

    assert_eq!(repeated.as_deref(), Some("é\u{FFFD}"));
}

#[test]
fn should_count_utf32_characters() {
    let text = Sample::new(TEXT, PositionEncoding::Utf32);
    let len = text.len();

    assert_eq!(len, 4);
}

#[test]
fn should_slice_utf32_characters() {
    let text = Sample::new(TEXT, PositionEncoding::Utf32);
    let sliced = text.slice(1..3);

    assert_eq!(sliced, "é😀");
}

#[test]
fn should_index_utf32_characters() {
    let text = Sample::new(TEXT, PositionEncoding::Utf32);
    let unit = text.at(2);

    assert_eq!(unit, "😀");
}

#[test]
fn should_repeat_utf32_text_a_fraction_of_times() {
    let text = Sample::new("😀b", PositionEncoding::Utf32);
    let repeated = text.repeat(1.5);

    assert_eq!(repeated.as_deref(), Some("😀b😀"));
}

#[test]
fn should_slice_ascii_as_unicode_is_sliced() {
    let text = Text::new("abcdef");
    let slices: Vec<(Text, Text)> = ENCODINGS
        .into_iter()
        .flat_map(|encoding| {
            let units = text.units(encoding);
            let unicode = TextUnitView {
                ascii: false,
                ..TextUnitView::new(&text, encoding)
            };

            (0..=units.len())
                .flat_map(|start| (start..=units.len()).map(move |end| start..end))
                .map(|range| (units.slice(range.clone()), unicode.slice(range)))
                .collect::<Vec<_>>()
        })
        .collect();

    for (sliced, unicode) in &slices {
        assert_eq!(&**sliced, &**unicode);
        assert_eq!(sliced.is_ascii(), str::is_ascii(sliced));
    }
}
