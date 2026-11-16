use gen_lsp_types as lsp;
use greybel_core::lexer::{AnyToken, TemplateScan};
use greybel_core::{Lexer, Punct, Span, TokenKind, TokenStream};

use super::encoder::TokenEncoder;
use super::roles::IdentifierRoles;
use super::token_type::TokenType;

pub(super) struct TokenScanner<'a> {
    roles: &'a IdentifierRoles<'a>,
    encoder: TokenEncoder<'a>,
}

impl<'a> TokenScanner<'a> {
    pub(super) fn new(roles: &'a IdentifierRoles<'a>, encoder: TokenEncoder<'a>) -> Self {
        Self { roles, encoder }
    }

    pub(super) fn into_tokens(self) -> Vec<lsp::SemanticToken> {
        self.encoder.into_tokens()
    }

    pub(super) fn scan(&mut self, lexer: &Lexer<'_>, stream: &mut TokenStream) {
        while let Ok(token) = lexer.next_token(stream) {
            let span = token.span();
            let token_type = match token.kind() {
                TokenKind::Eof => break,
                _ if token.as_template().is_some() => {
                    self.scan_template(lexer, span);
                    continue;
                }
                TokenKind::Keyword => TokenType::Keyword,
                TokenKind::StringLiteral => TokenType::String,
                TokenKind::NumericLiteral => TokenType::Number,
                TokenKind::BooleanLiteral | TokenKind::NilLiteral => TokenType::Constant,
                TokenKind::Comment => TokenType::Comment,
                TokenKind::Identifier => self.roles.of_token(span),
                TokenKind::Punctuator => Self::punctuator_type(&token),
                TokenKind::SliceOperator => TokenType::Punctuator,
                TokenKind::Eol | TokenKind::Invalid => continue,
            };

            self.encoder.push(span.start, span.end, token_type);
        }
    }

    fn scan_template(&mut self, lexer: &Lexer<'_>, span: Span) {
        let source = lexer.source();
        let Some(shape) = TemplateScan::new(source, true).template(span.start as usize) else {
            self.encoder.push(span.start, span.end, TokenType::String);

            return;
        };
        let mut text_start = span.start;

        for placeholder in shape.placeholders {
            self.encoder
                .push(text_start, placeholder.start, TokenType::String);

            let placeholder_lexer =
                Lexer::with_grammar(&source[..placeholder.end as usize], lexer.grammar());
            let mut stream = TokenStream::new_unsafe();

            stream.cursor.set(placeholder.start as usize);
            self.scan(&placeholder_lexer, &mut stream);
            text_start = placeholder.end;
        }

        self.encoder.push(text_start, span.end, TokenType::String);
    }

    fn punctuator_type(token: &AnyToken) -> TokenType {
        match token.punct() {
            Some(
                Punct::LParen
                | Punct::RParen
                | Punct::LBracket
                | Punct::RBracket
                | Punct::LBrace
                | Punct::RBrace
                | Punct::Comma
                | Punct::Dot,
            ) => TokenType::Punctuator,
            _ => TokenType::Operator,
        }
    }
}
