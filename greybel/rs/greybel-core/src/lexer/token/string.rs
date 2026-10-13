use std::borrow::Cow;

use greybel_symbols::delimiters::{ESCAPED_QUOTE, QUOTE};

use super::{PendingToken, Token, TokenKind};
use crate::ast::TextSpan;
use crate::diagnostics::LexerDiagnostic;
use crate::error::ParseError;
use crate::lexer::{LexResult, Lexer, TokenStream, is_end_of_line, offset};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringLiteral {
    pub(super) span: Span,
    pub(super) after_space: bool,
    pub(super) escaped: bool,
}

impl StringLiteral {
    #[must_use]
    pub fn unescaped_text<'s>(&self, source: &'s str) -> Cow<'s, str> {
        self.text().read(source)
    }

    #[must_use]
    pub fn raw<'s>(&self, source: &'s str) -> &'s str {
        self.span.bytes().map_or("", |bytes| &source[bytes])
    }

    #[must_use]
    pub fn text(&self) -> TextSpan {
        let quote = offset(QUOTE.len());
        let start = self.span.start + quote;
        let inner = Span::new(start, self.span.end.saturating_sub(quote).max(start));

        if self.escaped {
            TextSpan::with_escapes(inner)
        } else {
            TextSpan::new(inner)
        }
    }
}

impl Token for StringLiteral {
    fn kind(&self) -> TokenKind {
        TokenKind::StringLiteral
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        self.after_space
    }

    fn value<'s>(&self, source: &'s str) -> Cow<'s, str> {
        self.unescaped_text(source)
    }

    fn describe(&self, source: &str) -> String {
        format!("String({})", self.unescaped_text(source))
    }

    fn display_value<'s>(&self, source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed(self.raw(source))
    }

    fn as_string(&self) -> Option<&StringLiteral> {
        Some(self)
    }
}

impl<'src> Lexer<'src> {
    pub(crate) fn scan_string(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
    ) -> LexResult {
        stream.cursor.advance(QUOTE.len());

        let multiline = self.grammar().multiline_strings;
        let mut closed = false;
        let mut escaped = false;

        while !self.is_at_end(stream) {
            if self.starts_with(stream, ESCAPED_QUOTE) {
                escaped = true;
                stream.cursor.advance(ESCAPED_QUOTE.len());
            } else if self.starts_with(stream, QUOTE) {
                closed = true;
                break;
            } else if !multiline && is_end_of_line(self.code_at(stream, 0)) {
                break;
            } else {
                stream.cursor.advance(1);
            }
        }

        if !closed {
            let error =
                ParseError::from_lexer(LexerDiagnostic::UnclosedString, token_start.span(stream));

            return self.raise(stream, token_start, error);
        }

        stream.cursor.advance(QUOTE.len());

        Ok(
            token_start.finish(stream, |span, after_space| StringLiteral {
                span,
                after_space,
                escaped,
            }),
        )
    }
}
