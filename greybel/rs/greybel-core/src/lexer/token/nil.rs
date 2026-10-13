use std::borrow::Cow;

use greybel_symbols::literals::NULL;

use super::{Token, TokenKind};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NilLiteral {
    pub(super) span: Span,
    pub(super) after_space: bool,
}

impl NilLiteral {
    pub const DESCRIPTION: &'static str = "Keyword(null)";
}

impl Token for NilLiteral {
    fn kind(&self) -> TokenKind {
        TokenKind::NilLiteral
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        self.after_space
    }

    fn value<'s>(&self, _source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed(NULL)
    }

    fn describe(&self, _source: &str) -> String {
        Self::DESCRIPTION.into()
    }

    fn display_value<'s>(&self, source: &'s str) -> Cow<'s, str> {
        self.value(source)
    }
}
