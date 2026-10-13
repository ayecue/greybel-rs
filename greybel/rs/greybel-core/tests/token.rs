use std::collections::HashSet;

use greybel_core::TokenKind;

use crate::support::token::{assert_agrees, lex_all};

#[test]
fn should_answer_direct_checks_like_the_token_trait() {
    let source = "if a == 1 then x = \"s\" // c\ny = b[1:2] + null + true + f(.5)\nend if !";

    let tokens = lex_all(source);

    for token in &tokens {
        assert_agrees(token, source);
    }

    let kinds: HashSet<_> = tokens.iter().map(|token| token.kind()).collect();
    let all = [
        TokenKind::Eof,
        TokenKind::Eol,
        TokenKind::StringLiteral,
        TokenKind::NumericLiteral,
        TokenKind::BooleanLiteral,
        TokenKind::NilLiteral,
        TokenKind::Keyword,
        TokenKind::Identifier,
        TokenKind::Punctuator,
        TokenKind::SliceOperator,
        TokenKind::Comment,
        TokenKind::Invalid,
    ];

    for kind in all {
        assert!(kinds.contains(&kind), "source covers {kind:?}");
    }
}
