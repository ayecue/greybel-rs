use greybel_core::{Dialect, Lexer, Parser, ParserOptions, Version};

use crate::support::parser_stream::parsed_past_snapshot;

#[test]
fn should_parse_independently_over_one_lexer() {
    let source = "x = 1\nprint x";
    let lexer = Lexer::new(source, Dialect::MiniScript, Version::Latest);
    let parsers = [
        Parser::from_lexer(&lexer, ParserOptions::default()),
        Parser::from_lexer(&lexer, ParserOptions::default()),
    ];
    let expected = greybel_core::parse(source, ParserOptions::default())
        .unwrap()
        .to_string();

    let [first, second] = parsers.map(|parser| parser.parse().unwrap());

    assert_eq!(first.to_string(), second.to_string());
    assert_eq!(first.to_string(), expected);
}

#[test]
fn should_restore_the_token_of_the_snapshot() {
    let lexer = Lexer::new("a = 1\nb = 2\n", Dialect::MiniScript, Version::Latest);
    let (mut parser, snapshot, token) = parsed_past_snapshot(&lexer);

    parser.stream.restore(snapshot);

    assert_eq!(
        (
            parser.token().describe(parser.source()),
            parser.token().span()
        ),
        token
    );
}

#[test]
fn should_continue_parsing_after_a_restore() {
    let lexer = Lexer::new("a = 1\nb = 2\n", Dialect::MiniScript, Version::Latest);
    let (mut parser, snapshot, _) = parsed_past_snapshot(&lexer);

    parser.stream.restore(snapshot);

    parser.advance().unwrap();

    assert_eq!(parser.token().value(parser.source()), "=");
}

#[test]
fn should_report_lexer_errors_of_a_rolled_back_statement_once() {
    let source = "x = (1 .5. 2\ny = 3";

    let output = greybel_core::parse_unsafe(source, ParserOptions::default());

    let messages: Vec<_> = output
        .lexer_errors
        .iter()
        .map(greybel_core::ParseError::message)
        .collect();

    assert_eq!(messages, ["invalid numeric literal: .5."]);
    assert_eq!(output.errors.len(), 1);
}
