use greybel_core::parser::ParseSnapshot;
use greybel_core::{Lexer, Parser, ParserOptions, Span};

pub fn parsed_past_snapshot<'src>(
    lexer: &Lexer<'src>,
) -> (Parser<'src>, ParseSnapshot, (String, Span)) {
    let mut parser = Parser::from_lexer(lexer, ParserOptions::default());

    parser.advance().unwrap();

    let snapshot = parser.stream.snapshot();
    let token = (
        parser.token().describe(parser.source()),
        parser.token().span(),
    );

    while !parser.token().is_eof() {
        parser.advance().unwrap();
    }

    (parser, snapshot, token)
}
