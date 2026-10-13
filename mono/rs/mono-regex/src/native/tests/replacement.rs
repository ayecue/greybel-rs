#[path = "support/replacement.rs"]
mod support;

use support::{replace, template};

#[test]
fn should_translate_replacements_to_templates() {
    let cases = [
        ("(a)(b)", "$2$1x"),
        ("(?<n>a)(b)", "${n}$1"),
        ("(a)", "$10 $x ${y} $"),
        ("a", "$$|$&|$_"),
    ];
    let [numbered, named, unknown, special] =
        cases.map(|(pattern, replacement)| template(pattern, replacement));

    assert_eq!(numbered.as_deref(), Some("${2}${1}x"));
    assert_eq!(named.as_deref(), Some("${1}${2}"));
    assert_eq!(unknown.as_deref(), Some("$$10 $$x $${y} $$"));
    assert_eq!(special.as_deref(), Some("$$|${0}|in$$put"));
}

#[test]
fn should_not_template_the_text_before_the_match() {
    let (pattern, replacement) = ("a", "$`");
    let translated = template(pattern, replacement);

    assert_eq!(translated, None);
}

#[test]
fn should_substitute_groups_like_dotnet() {
    let cases = [
        ("(a)(b)", "xaby", "$2$1"),
        ("(a)", "a", "$1a"),
        ("(a)", "a", "$10"),
        ("(?<x>a)", "a", "[${x}]"),
        ("a", "a", "${nope}"),
        ("(?<x>a)(b)", "ab", "$1$2"),
    ];
    let [
        swapped,
        followed_by_text,
        unknown_number,
        named,
        unknown_name,
        numbered_after_named,
    ] = cases.map(|(pattern, input, replacement)| replace(pattern, input, replacement));

    assert_eq!(swapped, "xbay");
    assert_eq!(followed_by_text, "aa");
    assert_eq!(unknown_number, "$10");
    assert_eq!(named, "[a]");
    assert_eq!(unknown_name, "${nope}");
    assert_eq!(numbered_after_named, "ba");
}

#[test]
fn should_substitute_special_patterns_like_dotnet() {
    let cases = [
        ("b", "abc", "[$`|$&|$']"),
        ("b", "abc", "$$"),
        ("b", "abc", "$_"),
        ("(a)(b)", "ab", "$+"),
        ("b", "abc", "$x$"),
    ];
    let [around, dollar, whole_input, last_group, literal] =
        cases.map(|(pattern, input, replacement)| replace(pattern, input, replacement));

    assert_eq!(around, "a[a|b|c]c");
    assert_eq!(dollar, "a$c");
    assert_eq!(whole_input, "aabcc");
    assert_eq!(last_group, "b");
    assert_eq!(literal, "a$x$c");
}
