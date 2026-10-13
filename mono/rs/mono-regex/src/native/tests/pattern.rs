#[path = "support/pattern.rs"]
mod support;

use support::{NONE, reason, source};

use super::translate;
use crate::RegexOptions;
use crate::native::error::Reason;

#[test]
fn should_translate_anchors_like_dotnet() {
    let patterns = [
        ("^a$", NONE),
        ("^a$", RegexOptions::MULTILINE),
        (r"a\Z", NONE),
    ];
    let [anchored, multiline, end] = patterns.map(|(pattern, options)| source(pattern, options));

    assert_eq!(anchored, r"\Aa(?=\n?\z)");
    assert_eq!(multiline, "(?m:^)a(?m:$)");
    assert_eq!(end, r"a(?=\n?\z)");
}

#[test]
fn should_translate_the_options() {
    let patterns = [
        ("a.", RegexOptions::SINGLELINE),
        ("(a)", RegexOptions::EXPLICIT_CAPTURE),
        ("a b # c", RegexOptions::IGNORE_PATTERN_WHITESPACE),
        ("[a b]", RegexOptions::IGNORE_PATTERN_WHITESPACE),
        ("ab", RegexOptions::IGNORE_CASE),
    ];
    let [
        singleline,
        explicit,
        whitespace,
        class_whitespace,
        ignore_case,
    ] = patterns.map(|(pattern, options)| source(pattern, options));

    assert_eq!(singleline, "a(?s:.)");
    assert_eq!(explicit, "(?:a)");
    assert_eq!(whitespace, "ab");
    assert_eq!(class_whitespace, r"[a\ b]");
    assert_eq!(ignore_case, "(?i)ab");
}

#[test]
fn should_translate_quantifiers_like_dotnet() {
    let patterns = ["x{,2}y{", "a{2}b{1,}c{0,1}?", "a(?#c)*"];
    let [literal_braces, counted, after_comment] = patterns.map(|pattern| source(pattern, NONE));

    assert_eq!(literal_braces, r"x\{,2\}y\{");
    assert_eq!(counted, "a{2}b+c??");
    assert_eq!(after_comment, "a*");
}

#[test]
fn should_translate_character_classes_like_dotnet() {
    let patterns = [
        "[a-z-[aeiou]]",
        "[^a-z-[x]]",
        "[a&&b~~-]",
        "[[]",
        r"[\w\W\d]",
        r"\w",
    ];
    let [
        subtraction,
        negated_subtraction,
        operators,
        bracket,
        shorthands,
        word,
    ] = patterns.map(|pattern| source(pattern, NONE));

    assert_eq!(subtraction, "[[a-z]--[aeiou]]");
    assert_eq!(negated_subtraction, "[[^a-z]--[x]]");
    assert_eq!(operators, r"[a\&\&b\~\~\-]");
    assert_eq!(bracket, r"[\[]");
    assert_eq!(
        shorthands,
        r"[\p{L}\p{Mn}\p{Nd}\p{Pc}[^\p{L}\p{Mn}\p{Nd}\p{Pc}]\d]"
    );
    assert_eq!(word, r"[\p{L}\p{Mn}\p{Nd}\p{Pc}]");
}

#[test]
fn should_translate_escapes_and_literals_like_dotnet() {
    let patterns = [r"\x41\u00e9\0\cA", r"\uD83D\uDE00", r"\10\101", "<a>"];
    let [escapes, surrogates, octal, angle_brackets] =
        patterns.map(|pattern| source(pattern, NONE));

    assert_eq!(escapes, r"Aé\x{0}\x{1}");
    assert_eq!(surrogates, "😀");
    assert_eq!(octal, r"\x{8}A");
    assert_eq!(angle_brackets, "<a>");
}

#[test]
fn should_scope_inline_options_like_dotnet() {
    let patterns = ["a(?i)b(?-i:c)", "(?i:a)b", "(a(?i)b|c)d"];
    let [switched, group, alternation] = patterns.map(|pattern| source(pattern, NONE));

    assert_eq!(switched, "a(?i:b)(?:c)");
    assert_eq!(group, "(?:(?i:a))b");
    assert_eq!(alternation, "(a(?i:b)|(?i:c))d");
}

#[test]
fn should_renumber_named_backreferences() {
    let pattern = "(?<x>a)(b)\\k<x>\\1";
    let translated = source(pattern, NONE);

    assert_eq!(translated, r"(a)(b)\k<1>\k<2>");
}

#[test]
fn should_translate_conditionals_like_dotnet() {
    let patterns = [
        "(?(1)a|b)(c)",
        "(?(a)b|c)",
        "(?(?!a)b)",
        "(?(1)(?i)b|)(a)",
        "(?(a)(b(?i)c))",
    ];
    let [
        on_group,
        on_expression,
        on_lookahead,
        inline_option,
        nested_option,
    ] = patterns.map(|pattern| source(pattern, NONE));

    assert_eq!(on_group, "(?(1)a|b)(c)");
    assert_eq!(on_expression, "(?((?=(?:a)))b|c)");
    assert_eq!(on_lookahead, "(?((?=(?!a)))b)");
    assert_eq!(inline_option, "(?(1)(?i:b)|)(a)");
    assert_eq!(nested_option, "(?((?=(?:a)))(b(?i:c)))");
}

#[test]
fn should_number_groups_like_dotnet() {
    let pattern = "(?<x>a)(b)(?<y>c)(d)";
    let groups = translate(pattern, RegexOptions::empty())
        .expect("translates")
        .groups;

    assert_eq!(groups.index(1), Some(2));
    assert_eq!(groups.index(2), Some(4));
    assert_eq!(groups.number("x"), Some(3));
    assert_eq!(groups.index(3), Some(1));
    assert_eq!(groups.number("y"), Some(4));
    assert_eq!(groups.last(), 4);
}

#[test]
fn should_number_a_name_after_explicit_numbers() {
    let pattern = "(?<2>a)(b)(?<x>c)";
    let groups = translate(pattern, RegexOptions::empty())
        .expect("translates")
        .groups;

    assert_eq!(groups.number("x"), Some(3));
}

#[test]
fn should_reject_unbalanced_parens_like_dotnet() {
    let patterns = ["(", ")"];
    let [unclosed, unopened] = patterns.map(reason);

    assert_eq!(unclosed, Reason::NotEnoughParens);
    assert_eq!(unopened, Reason::TooManyParens);
}

#[test]
fn should_reject_misplaced_quantifiers_like_dotnet() {
    let patterns = [
        "*a", "a|*", "a*(?i)*", "a**", "a*(?#c)+", "a{2}{3}", "a{3,2}",
    ];
    let [
        leading,
        after_bar,
        after_option,
        nested,
        after_comment,
        nested_braces,
        range,
    ] = patterns.map(reason);

    assert_eq!(leading, Reason::QuantifyAfterNothing);
    assert_eq!(after_bar, Reason::QuantifyAfterNothing);
    assert_eq!(after_option, Reason::QuantifyAfterNothing);
    assert_eq!(nested, Reason::NestedQuantify('*'));
    assert_eq!(after_comment, Reason::NestedQuantify('+'));
    assert_eq!(nested_braces, Reason::NestedQuantify('{'));
    assert_eq!(range, Reason::IllegalRange);
}

#[test]
fn should_reject_malformed_classes_like_dotnet() {
    let patterns = ["[a", "*[a", "[z-a]", r"[a-\d]", "[a-[b]c]"];
    let [
        unterminated,
        before_quantifier,
        reversed,
        shorthand_range,
        subtraction,
    ] = patterns.map(reason);

    assert_eq!(unterminated, Reason::UnterminatedBracket);
    assert_eq!(before_quantifier, Reason::UnterminatedBracket);
    assert_eq!(reversed, Reason::ReversedCharRange);
    assert_eq!(shorthand_range, Reason::BadClassInCharRange('d'));
    assert_eq!(subtraction, Reason::SubtractionMustBeLast);
}

#[test]
fn should_reject_malformed_escapes_like_dotnet() {
    let patterns = [
        r"*\q", r"a\", r"\q", r"\x4", r"\c", r"\p{L", r"\pL", r"\pLu}", r"\uD83D",
    ];
    let [
        before_quantifier,
        trailing,
        unknown,
        short_hex,
        no_control,
        unclosed_category,
        bare_category,
        unopened_category,
        lone_surrogate,
    ] = patterns.map(reason);

    assert_eq!(before_quantifier, Reason::UnrecognizedEscape('q'));
    assert_eq!(trailing, Reason::IllegalEndEscape);
    assert_eq!(unknown, Reason::UnrecognizedEscape('q'));
    assert_eq!(short_hex, Reason::TooFewHex);
    assert_eq!(no_control, Reason::MissingControl);
    assert_eq!(unclosed_category, Reason::IncompleteSlashP);
    assert_eq!(bare_category, Reason::IncompleteSlashP);
    assert_eq!(unopened_category, Reason::MalformedSlashP);
    assert_eq!(lone_surrogate, Reason::LoneSurrogate);
}

#[test]
fn should_reject_undefined_references_like_dotnet() {
    let patterns = [r"\2(a)", r"\k<x>", r"\k"];
    let [number, name, bare] = patterns.map(reason);

    assert_eq!(number, Reason::UndefinedBackref(2));
    assert_eq!(name, Reason::UndefinedNameRef("x".into()));
    assert_eq!(bare, Reason::MalformedNameRef);
}

#[test]
fn should_reject_malformed_groups_like_dotnet() {
    let patterns = [
        "(?<0>a)",
        "(?<a!>a)",
        "(?'a'=b)(?'=b)",
        "(?Q)",
        "(?#a",
        "(?<a-",
    ];
    let [
        zero,
        invalid_name,
        empty_name,
        unknown,
        unterminated_comment,
        unterminated_balance,
    ] = patterns.map(reason);

    assert_eq!(zero, Reason::CapnumNotZero);
    assert_eq!(invalid_name, Reason::InvalidGroupName);
    assert_eq!(empty_name, Reason::UnrecognizedGroup);
    assert_eq!(unknown, Reason::UnrecognizedGroup);
    assert_eq!(unterminated_comment, Reason::UnterminatedComment);
    assert_eq!(unterminated_balance, Reason::UnrecognizedGroup);
}

#[test]
fn should_reject_malformed_conditionals_like_dotnet() {
    let patterns = [
        "(?(1)a|b|c)(d)",
        "(?(2)a)(b)",
        "(?(2a)b)",
        "(?(?<x>a)b)",
        "(?(?#a)b)",
        "(?(?i)a|b)",
        "(?(a)(?i:b))",
    ];
    let [
        alternates,
        undefined,
        malformed,
        capture,
        comment,
        inline_option,
        option_group,
    ] = patterns.map(reason);

    assert_eq!(alternates, Reason::TooManyAlternates);
    assert_eq!(undefined, Reason::UndefinedReference(2));
    assert_eq!(malformed, Reason::MalformedReference(2));
    assert_eq!(capture, Reason::AlternationCantCapture);
    assert_eq!(comment, Reason::AlternationCantHaveComment);
    assert_eq!(inline_option, Reason::UnrecognizedGroup);
    assert_eq!(option_group, Reason::UnrecognizedGroup);
}

#[test]
fn should_reject_balancing_groups() {
    let patterns = ["(?<a-b>x)", "(?<b>x)(?<a-b>y)", "(?<-1>x)(y)"];
    let [undefined, named, numbered] = patterns.map(reason);

    assert_eq!(undefined, Reason::UndefinedNameRef("b".into()));
    assert_eq!(named, Reason::BalancingGroup);
    assert_eq!(numbered, Reason::BalancingGroup);
}

#[test]
fn should_reject_duplicate_groups() {
    let patterns = ["(?<x>a)(?<x>b)", "(?<1>a)(b)"];
    let [named, numbered] = patterns.map(reason);

    assert_eq!(named, Reason::DuplicateGroup("x".into()));
    assert_eq!(numbered, Reason::DuplicateGroup("1".into()));
}

#[test]
fn should_describe_a_rejection_like_dotnet() {
    let pattern = "(";
    let error = translate(pattern, RegexOptions::empty()).expect_err("rejected");

    assert_eq!(error.to_string(), "parsing \"(\" - Not enough )'s.");
}
