use std::borrow::Cow;

use super::{AnyToken, PendingToken, Token, TokenKind};
use crate::diagnostics::Diagnostic;
use crate::error::ParseError;
use crate::lexer::{LexResult, Lexer, TokenStream};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Invalid {
    pub(super) span: Span,
}

impl Invalid {
    pub const DESCRIPTION: &'static str = "Unknown";
}

impl Token for Invalid {
    fn kind(&self) -> TokenKind {
        TokenKind::Invalid
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        false
    }

    fn value<'s>(&self, _source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed("")
    }

    fn describe(&self, source: &str) -> String {
        let text = source
            .get(self.span.start as usize..self.span.end as usize)
            .unwrap_or_default();

        format!("{}({text})", Self::DESCRIPTION)
    }
}

impl<'src> Lexer<'src> {
    pub(crate) fn scan_unknown(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
    ) -> AnyToken {
        let width = self.source[stream.cursor.index()..]
            .chars()
            .next()
            .map_or(1, char::len_utf8);

        stream.cursor.advance(width);

        token_start.finish(stream, |span, _| Invalid { span })
    }

    pub fn raise(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
        error: ParseError,
    ) -> LexResult {
        stream.diagnostics.report(error)?;

        Ok(token_start.finish(stream, |span, _| Invalid { span }))
    }

    pub fn raise_at(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
        diagnostic: impl Into<Diagnostic>,
    ) -> LexResult {
        let error = ParseError::from_lexer(diagnostic, token_start.span(stream));

        self.raise(stream, token_start, error)
    }
}
