use std::borrow::Cow;
use std::fmt;

use greybel_symbols::literals::{FALSE, TRUE};

use super::{Token, TokenKind};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BooleanLiteral {
    pub(super) span: Span,
    pub(super) after_space: bool,
    pub(super) value: bool,
}

impl BooleanLiteral {
    #[must_use]
    pub fn parse(word: &str) -> Option<bool> {
        match word {
            TRUE => Some(true),
            FALSE => Some(false),
            _ => None,
        }
    }

    #[must_use]
    pub fn boolean(&self) -> bool {
        self.value
    }

    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        if self.value { TRUE } else { FALSE }
    }
}

impl fmt::Display for BooleanLiteral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Token for BooleanLiteral {
    fn kind(&self) -> TokenKind {
        TokenKind::BooleanLiteral
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        self.after_space
    }

    fn value<'s>(&self, _source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed(self.as_str())
    }

    fn describe(&self, _source: &str) -> String {
        format!("Keyword({})", self.as_str())
    }

    fn display_value<'s>(&self, source: &'s str) -> Cow<'s, str> {
        self.value(source)
    }

    fn as_boolean(&self) -> Option<&BooleanLiteral> {
        Some(self)
    }
}
