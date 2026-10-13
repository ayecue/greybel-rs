use std::borrow::Cow;

use greybel_symbols::delimiters::SLICE_SEPARATOR;

use super::{AnyToken, PendingToken, Token, TokenKind};
use crate::lexer::{Lexer, TokenStream};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceOperator {
    pub(super) span: Span,
    pub(super) after_space: bool,
}

impl SliceOperator {
    pub const DESCRIPTION: &'static str = "Colon";
}

impl Token for SliceOperator {
    fn kind(&self) -> TokenKind {
        TokenKind::SliceOperator
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        self.after_space
    }

    fn value<'s>(&self, _source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed(SLICE_SEPARATOR)
    }

    fn describe(&self, _source: &str) -> String {
        Self::DESCRIPTION.into()
    }
}

impl<'src> Lexer<'src> {
    pub(crate) fn scan_slice_operator(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
    ) -> AnyToken {
        stream.cursor.advance(SLICE_SEPARATOR.len());

        token_start.finish(stream, |span, after_space| SliceOperator {
            span,
            after_space,
        })
    }
}
