//! Snapshots lexer and parser output for TypeScript fixtures; known differences are documented in `PORTING.md`.

use support::cases::{GREYBEL, GREYSCRIPT, MINISCRIPT, outputs};

use crate::support;

#[test]
fn should_lex_and_parse_miniscript() {
    let (lexed, parsed) = outputs(&MINISCRIPT);

    insta::assert_yaml_snapshot!("miniscript_lexer", lexed);
    insta::assert_yaml_snapshot!("miniscript_parser", parsed);
}

#[test]
fn should_lex_and_parse_greybel() {
    let (lexed, parsed) = outputs(&GREYBEL);

    insta::assert_yaml_snapshot!("greybel_lexer", lexed);
    insta::assert_yaml_snapshot!("greybel_parser", parsed);
}

#[test]
fn should_lex_and_parse_greyscript() {
    let (lexed, parsed) = outputs(&GREYSCRIPT);

    insta::assert_yaml_snapshot!("greyscript_lexer", lexed);
    insta::assert_yaml_snapshot!("greyscript_parser", parsed);
}
