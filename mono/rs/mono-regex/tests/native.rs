use mono_regex::{Match, NativeRegexEngine, RegexEngine, RegexError, RegexOptions};
use support::native::{NONE, is_match, split};

use crate::support;

#[test]
fn should_split_on_a_separator() {
    let cases = [("a,b,,c", ","), ("abc", ""), ("abc", "x")];
    let [separated, empty, missing] = cases.map(|(input, pattern)| split(input, pattern, NONE));

    assert_eq!(separated, ["a", "b", "", "c"]);
    assert_eq!(empty, ["", "a", "b", "c", ""]);
    assert_eq!(missing, ["abc"]);
}

#[test]
fn should_keep_captured_groups_when_splitting() {
    let cases = [("a-b", "(-)"), ("a+b", "(-)|(\\+)"), ("a-=b", "(?<x>-)(=)")];
    let [captured, alternative, named] = cases.map(|(input, pattern)| split(input, pattern, NONE));

    assert_eq!(captured, ["a", "-", "b"]);
    assert_eq!(alternative, ["a", "+", "b"]);
    assert_eq!(named, ["a", "=", "-", "b"]);
}

#[test]
fn should_drop_unnamed_groups_with_explicit_capture() {
    let explicit = RegexOptions::EXPLICIT_CAPTURE;
    let patterns = ["(-)", "(?<d>-)"];
    let [unnamed, named] = patterns.map(|pattern| split("a-b", pattern, explicit));

    assert_eq!(unnamed, ["a", "b"]);
    assert_eq!(named, ["a", "-", "b"]);
}

#[test]
fn should_apply_the_options() {
    let cases = [
        ("xabcx", "ABC", RegexOptions::IGNORE_CASE),
        ("xabcx", "ABC", NONE),
        ("a\nb\nc", "^b$", RegexOptions::MULTILINE),
        ("a\nb", "a.b", RegexOptions::SINGLELINE),
        ("ab", "a b", RegexOptions::IGNORE_PATTERN_WHITESPACE),
    ];
    let [
        ignore_case,
        case_sensitive,
        multiline,
        singleline,
        whitespace,
    ] = cases.map(|(input, pattern, options)| is_match(input, pattern, options));

    assert!(ignore_case);
    assert!(!case_sensitive);
    assert!(multiline);
    assert!(singleline);
    assert!(whitespace);
}

#[test]
fn should_apply_inline_options() {
    let inputs = ["aBc", "aBC"];
    let [switched_on, switched_off] = inputs.map(|input| is_match(input, "a(?i)b(?-i)c", NONE));

    assert!(switched_on);
    assert!(!switched_off);
}

#[test]
fn should_read_patterns_like_dotnet() {
    let cases = [
        ("ab\n", "b$"),
        ("ab\n\n", "b$"),
        ("a{,2}", r"a{,2}"),
        ("b", "[a-z-[aeiou]]"),
        ("e", "[a-z-[aeiou]]"),
        ("&", "[&&]"),
        ("aa", r"(a)\1"),
        ("abab", r"(?<x>a)(b)\k<x>\1"),
        ("cd", "(?(a)ab|cd)"),
        ("ad", "(?(a)ab|cd)"),
    ];
    let [
        end_before_newline,
        end_before_newlines,
        literal_braces,
        subtracted_kept,
        subtracted_removed,
        ampersands,
        backreference,
        named_backreference,
        conditional_no,
        conditional_neither,
    ] = cases.map(|(input, pattern)| is_match(input, pattern, NONE));

    assert!(end_before_newline);
    assert!(!end_before_newlines);
    assert!(literal_braces);
    assert!(subtracted_kept);
    assert!(!subtracted_removed);
    assert!(ampersands);
    assert!(backreference);
    assert!(named_backreference);
    assert!(conditional_no);
    assert!(!conditional_neither);
}

#[test]
fn should_match_a_conditional_on_a_named_group() {
    let engine = NativeRegexEngine::default();
    let found = engine.matches("ab c", "(?<x>a)(?(x)b|c)", NONE);

    assert_eq!(
        found,
        Ok(vec![Match {
            index: 0,
            value: "ab".into()
        }])
    );
}

#[test]
fn should_count_match_indices_in_utf16_units() {
    let engine = NativeRegexEngine::default();
    let found = engine.matches("aé😀b😀", "😀|b", NONE).expect("matches");
    let indices: Vec<usize> = found.iter().map(|found| found.index).collect();

    assert_eq!(indices, [2, 4, 5]);
}

#[test]
fn should_give_invalid_patterns_dotnet_messages() {
    let engine = NativeRegexEngine::default();
    let result = engine.is_match("", "a**", NONE);

    assert_eq!(
        result,
        Err(RegexError::Failure(
            "parsing \"a**\" - Nested quantifier *.".into()
        ))
    );
}

#[test]
fn should_time_out_on_catastrophic_patterns() {
    let engine = NativeRegexEngine::default();
    let input = format!("{}b", "a".repeat(40));
    let result = engine.is_match(&input, "(a|aa)+(?=b)c", NONE);

    assert_eq!(result, Err(RegexError::Timeout));
}
