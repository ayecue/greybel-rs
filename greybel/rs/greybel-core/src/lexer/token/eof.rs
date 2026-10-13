use std::borrow::Cow;

use super::{AnyToken, PendingToken, Token, TokenKind};
use crate::lexer::{Lexer, TokenStream};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Eof {
    pub(super) span: Span,
    pub(super) after_space: bool,
}

impl Eof {
    /// MiniScript's lexer appends an end-of-line token, which end-of-input errors identify.
    pub const DESCRIPTION: &'static str = "EOL";
    pub const VALUE: &'static str = "<eof>";

    pub(crate) fn new(span: Span) -> Self {
        Self {
            span,
            after_space: false,
        }
    }
}

impl Token for Eof {
    fn kind(&self) -> TokenKind {
        TokenKind::Eof
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        self.after_space
    }

    fn value<'s>(&self, _source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed(Self::VALUE)
    }

    fn describe(&self, _source: &str) -> String {
        Self::DESCRIPTION.into()
    }
}

impl<'src> Lexer<'src> {
    pub(crate) fn scan_eof(&self, stream: &TokenStream, token_start: PendingToken) -> AnyToken {
        token_start.finish(stream, |span, after_space| Eof { span, after_space })
    }
}
