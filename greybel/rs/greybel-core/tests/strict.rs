//! Tests that strict mode parses only what each dialect's reference implementation compiles.

use greybel_core::{Dialect, Version};

use crate::support::strict::{GREYBEL_SYNTAX, parses};

#[test]
fn should_parse_greybel_syntax_in_greyscript_only_outside_strict_mode() {
    let accepted: Vec<bool> = GREYBEL_SYNTAX
        .iter()
        .map(|source| parses(source, Dialect::GreyScript, Version::V1_6, false))
        .collect();
    let strict: Vec<&str> = GREYBEL_SYNTAX
        .into_iter()
        .filter(|source| parses(source, Dialect::GreyScript, Version::V1_6, true))
        .collect();

    assert!(accepted.iter().all(|accepted| *accepted), "{accepted:?}");
    assert_eq!(strict, Vec::<&str>::new());
}

#[test]
fn should_parse_what_grey_hack_compiles_in_strict_greyscript() {
    let sources = [
        "debugger = 2\nprint debugger",
        "import_code(\"/lib.src\")",
        "x = [1,\n2]",
        "f = function(a = -1, b = \"s\", c = null, d = (2))\nend function",
    ];

    let rejected: Vec<&str> = sources
        .into_iter()
        .filter(|source| !parses(source, Dialect::GreyScript, Version::V1_6, true))
        .collect();

    assert_eq!(rejected, Vec::<&str>::new());
}

/// Grey Hack 0.9.7373 ends a string at a line break and rejects a parameter default it computes into a temporary.
#[test]
fn should_reject_what_grey_hack_no_longer_compiles_in_strict_greyscript() {
    let sources = [
        "f = function(a = 1 + 1)\nend function",
        "f = function(a = b)\nend function",
        "f = function(a = [1])\nend function",
        "f = function(a = {})\nend function",
        "f = function(a = b.c)\nend function",
        "f = function(a = b[0])\nend function",
        "f = function(a = -b)\nend function",
        "f = function(a = function()\nend function)\nend function",
        "s = \"a\nb\"",
    ];

    let accepted: Vec<&str> = sources
        .into_iter()
        .filter(|source| parses(source, Dialect::GreyScript, Version::V1_6, true))
        .collect();

    assert_eq!(accepted, Vec::<&str>::new());
}

/// Grey Hack compiles an `@` reference as a default, but the parameter then holds the unevaluated reference itself
/// (`ValVar`), which prints as `@g` and is no function: GreyScript and every strict mode reject it, and only Greybel's
/// own dialect takes it.
#[test]
fn should_reject_reference_defaults_but_in_greybel() {
    let source = "f = function(e = @g, h = @g.k)\nend function";

    let accepted = [
        (Dialect::GreyScript, false),
        (Dialect::GreyScript, true),
        (Dialect::Greybel, false),
        (Dialect::Greybel, true),
    ]
    .map(|(dialect, strict)| parses(source, dialect, Version::V1_6, strict));

    assert_eq!(accepted, [false, false, true, false]);
}

#[test]
fn should_parse_compound_assignments_in_strict_mode_from_miniscript_1_6_2() {
    let source = "x = 1\nx += 1";

    let versions = [Version::V1_6, Version::Latest]
        .map(|version| parses(source, Dialect::MiniScript, version, true));

    assert_eq!(versions, [false, true]);
}

#[test]
fn should_keep_greybel_syntax_but_not_its_conveniences_in_strict_greybel() {
    let syntax = [
        "x = 5 & 3",
        "x = #line",
        "#include \"/lib.src\"",
        "/* block */\nx = 1",
    ];
    let conveniences = [
        "f = function(a)\nend function\nf(1,)",
        "while true print 1",
        "f = function(a)\nend function\nf(1\n)",
    ];

    let syntax = syntax.map(|source| parses(source, Dialect::Greybel, Version::Latest, true));
    let conveniences =
        conveniences.map(|source| parses(source, Dialect::Greybel, Version::Latest, true));

    assert_eq!(syntax, [true; 4]);
    assert_eq!(conveniences, [false; 3]);
}
