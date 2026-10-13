use greybel_core::lexer::token::AnyToken;
use greybel_core::{Dialect, Keyword, Lexer, Punct, Token, TokenStream, Version};

pub fn lex_all(source: &str) -> Vec<AnyToken> {
    let lexer = Lexer::new(source, Dialect::MiniScript, Version::Latest);
    let mut stream = TokenStream::new_unsafe();
    let mut tokens = Vec::new();

    loop {
        lexer
            .advance(&mut stream)
            .expect("an unsafe lexer does not fail");

        let token = *stream.current();
        let eof = token.is_eof();

        tokens.push(token);
        if eof {
            break;
        }
    }

    tokens
}

pub fn assert_agrees(token: &AnyToken, source: &str) {
    let dynamic: &dyn Token = token.as_token();

    assert_eq!(token.kind(), dynamic.kind(), "{token:?}");
    assert_eq!(token.span(), dynamic.span(), "{token:?}");
    assert_eq!(token.keyword(), dynamic.keyword(), "{token:?}");
    assert_eq!(token.punct(), dynamic.punct(), "{token:?}");
    assert_eq!(token.is_eof(), dynamic.is_eof(), "{token:?}");
    assert_eq!(token.is_eol(), dynamic.is_eol(), "{token:?}");
    assert_eq!(token.is_comment(), dynamic.is_comment(), "{token:?}");
    assert_eq!(token.is_identifier(), dynamic.is_identifier(), "{token:?}");
    assert_eq!(
        token.as_identifier().map(|t| t.name(source)),
        dynamic.as_identifier().map(|t| t.name(source)),
        "{token:?}"
    );
    for keyword in [Keyword::If, Keyword::EndIf, Keyword::Not] {
        assert_eq!(token.is_keyword(keyword), dynamic.is_keyword(keyword));
    }

    for punct in [Punct::LParen, Punct::Equal, Punct::Dot] {
        assert_eq!(token.is_punct(punct), dynamic.is_punct(punct));
    }
}
