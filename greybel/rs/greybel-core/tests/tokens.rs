use greybel_core::{Dialect, ErrorCode, Lexer, Version};

#[test]
fn should_end_before_eof_and_stay_ended() {
    let lexer = Lexer::new("a", Dialect::MiniScript, Version::Latest);
    let mut tokens = lexer.tokens();

    let items: Vec<_> = (0..3).map(|_| tokens.next()).collect();

    assert!(
        matches!(&items[0], Some(Ok(token)) if token.as_identifier().map(|i| i.name("a")) == Some("a"))
    );
    assert!(items[1].is_none());
    assert!(items[2].is_none());
}

#[test]
fn should_stop_strict_iteration_after_the_first_error() {
    let lexer = Lexer::new("a .5. b", Dialect::MiniScript, Version::Latest);

    let items: Vec<_> = lexer.tokens().collect();

    assert_eq!(items.len(), 2);
    assert!(items[0].is_ok());
    assert_eq!(
        items[1].as_ref().unwrap_err().code(),
        ErrorCode::InvalidNumericLiteral
    );
}

#[test]
fn should_continue_unsafe_iteration_past_errors() {
    let lexer = Lexer::new("a .5. b", Dialect::MiniScript, Version::Latest);
    let mut tokens = lexer.tokens_unsafe();

    let kinds: Vec<_> = tokens.by_ref().map(|t| t.unwrap().type_name()).collect();

    assert_eq!(kinds, ["Identifier", "Invalid", "Identifier"]);
    assert_eq!(tokens.stream().errors().len(), 1);
}

#[test]
fn should_take_an_unknown_character_as_a_token_without_an_error() {
    let lexer = Lexer::new("a ! $", Dialect::MiniScript, Version::Latest);
    let mut tokens = lexer.tokens_unsafe();

    let described: Vec<_> = tokens
        .by_ref()
        .map(|token| token.unwrap().describe(lexer.source()))
        .collect();

    assert_eq!(described, ["Identifier(a)", "Unknown(!)", "Unknown($)"]);
    assert!(tokens.stream().errors().is_empty());
}
