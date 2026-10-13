//! Tests regular expressions compiled to deterministic transition tables.

use duplex_protocol::PatternTable;

#[test]
fn should_match_only_entire_texts() {
    let table = PatternTable::compile("[a-z0-9_]+").unwrap();

    let matches: Vec<bool> = ["user_1", "", "User", "a b", "abc!"]
        .iter()
        .map(|text| table.matches(text.as_bytes()))
        .collect();

    assert_eq!(matches, [true, false, false, false, false]);
}

#[test]
fn should_match_text_beyond_ascii() {
    let table = PatternTable::compile(r"\p{L}{2,3}").unwrap();

    let matches: Vec<bool> = ["éß", "日本語", "a", "日本語日"]
        .iter()
        .map(|text| table.matches(text.as_bytes()))
        .collect();

    assert_eq!(matches, [true, true, false, false]);
}

#[test]
fn should_match_alternatives_where_one_is_a_prefix_of_another() {
    let table = PatternTable::compile("ab|abcd").unwrap();

    let matches: Vec<bool> = ["ab", "abc", "abcd"]
        .iter()
        .map(|text| table.matches(text.as_bytes()))
        .collect();

    assert_eq!(matches, [true, false, true]);
}

#[test]
fn should_refuse_what_isnt_a_pattern() {
    let pattern = "(a";

    let error = PatternTable::compile(pattern).unwrap_err();

    assert!(
        error.starts_with("isn't a pattern a table can match"),
        "{error}"
    );
}

#[test]
fn should_refuse_a_table_past_its_size() {
    let pattern = r"\p{L}{1,16}";

    let error = PatternTable::compile(pattern).unwrap_err();

    assert!(
        error.starts_with("takes a table of more than 256 KiB"),
        "{error}"
    );
}
