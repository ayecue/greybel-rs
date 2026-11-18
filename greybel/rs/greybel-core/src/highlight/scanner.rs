use super::roles::IdentifierRoles;
use super::{HighlightKind, HighlightToken};
use crate::lexer::{AnyToken, TemplateScan};
use crate::{Lexer, Punct, Span, TokenKind, TokenStream};

pub(super) struct TokenScanner<'a> {
    roles: &'a IdentifierRoles<'a>,
    library: Option<&'a dyn Fn(&str) -> bool>,
    tokens: Vec<HighlightToken>,
}

impl<'a> TokenScanner<'a> {
    pub(super) fn new(
        roles: &'a IdentifierRoles<'a>,
        library: Option<&'a dyn Fn(&str) -> bool>,
    ) -> Self {
        Self {
            roles,
            library,
            tokens: Vec::new(),
        }
    }

    pub(super) fn into_tokens(self) -> Vec<HighlightToken> {
        self.tokens
    }

    pub(super) fn scan(&mut self, lexer: &Lexer<'_>, stream: &mut TokenStream) {
        while let Ok(token) = lexer.next_token(stream) {
            let span = token.span();
            let kind = match token.kind() {
                TokenKind::Eof => break,
                _ if token.as_template().is_some() => {
                    self.scan_template(lexer, span);
                    continue;
                }
                TokenKind::Keyword => HighlightKind::Keyword,
                TokenKind::StringLiteral => HighlightKind::String,
                TokenKind::NumericLiteral => HighlightKind::Number,
                TokenKind::BooleanLiteral | TokenKind::NilLiteral => HighlightKind::Constant,
                TokenKind::Comment => HighlightKind::Comment,
                TokenKind::Identifier => self.roles.of_token(span),
                TokenKind::Punctuator => Self::punctuator_kind(&token),
                TokenKind::SliceOperator => HighlightKind::Punctuator,
                TokenKind::Eol | TokenKind::Invalid => continue,
            };

            let library = matches!(
                kind,
                HighlightKind::Variable | HighlightKind::Function | HighlightKind::Property
            ) && self.library.is_some_and(|is_library| {
                is_library(&lexer.source()[span.start as usize..span.end as usize])
            });

            self.push_token(span.start, span.end, kind, library);
        }
    }

    fn scan_template(&mut self, lexer: &Lexer<'_>, span: Span) {
        let source = lexer.source();
        let Some(shape) = TemplateScan::new(source, true).template(span.start as usize) else {
            self.push(span.start, span.end, HighlightKind::String);

            return;
        };
        let mut text_start = span.start;

        for placeholder in shape.placeholders {
            self.push(text_start, placeholder.start, HighlightKind::String);

            let placeholder_lexer =
                Lexer::with_grammar(&source[..placeholder.end as usize], lexer.grammar());
            let mut stream = TokenStream::new_unsafe();

            stream.cursor.set(placeholder.start as usize);
            self.scan(&placeholder_lexer, &mut stream);
            text_start = placeholder.end;
        }

        self.push(text_start, span.end, HighlightKind::String);
    }

    fn push(&mut self, start: u32, end: u32, kind: HighlightKind) {
        self.push_token(start, end, kind, false);
    }

    fn push_token(&mut self, start: u32, end: u32, kind: HighlightKind, library: bool) {
        if start < end {
            self.tokens.push(HighlightToken {
                span: Span::new(start, end),
                kind,
                library,
            });
        }
    }

    fn punctuator_kind(token: &AnyToken) -> HighlightKind {
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
            ) => HighlightKind::Punctuator,
            _ => HighlightKind::Operator,
        }
    }
}
