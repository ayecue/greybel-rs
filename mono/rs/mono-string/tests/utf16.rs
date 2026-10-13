use std::borrow::Cow;

use mono_string::utf16::*;

#[test]
fn should_count_a_surrogate_pair_as_two_units() {
    let text = "a\u{1F600}b";
    let units = len(text);

    assert_eq!(units, 4);
}

#[test]
fn should_replace_the_halves_of_a_split_pair() {
    let text = "a\u{1F600}b";
    let ranges = [0..2, 2..4, 2..3, 1..3];
    let [leading, trailing, lone, complete] = ranges.map(|range| substring(text, range));

    assert_eq!(leading, "a\u{FFFD}");
    assert_eq!(trailing, "\u{FFFD}b");
    assert_eq!(lone, "\u{FFFD}");
    assert_eq!(complete, "\u{1F600}");
}

#[test]
fn should_borrow_a_substring_of_whole_characters() {
    let text = "a\u{1F600}b";
    let slice = substring(text, 1..4);

    assert!(matches!(slice, Cow::Borrowed("\u{1F600}b")));
}
