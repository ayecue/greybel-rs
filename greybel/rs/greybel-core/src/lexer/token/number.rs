use std::borrow::Cow;

use super::{PendingToken, Token, TokenKind};
use crate::diagnostics::LexerDiagnostic;
use crate::lexer::{LexResult, Lexer, TokenStream};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumericLiteral {
    pub(super) span: Span,
    pub(super) after_space: bool,
}

impl NumericLiteral {
    pub const DECIMAL_POINT: u8 = b'.';
    pub const EXPONENT_MARKERS: [u8; 2] = *b"eE";
    pub const EXPONENT_SIGNS: [u8; 2] = *b"-+";
}

impl NumericLiteral {
    #[must_use]
    pub fn number(&self, source: &str) -> f64 {
        self.raw(source)
            .parse()
            .expect("the lexer only accepts numbers in the shape `str::parse` reads")
    }

    #[must_use]
    pub fn raw<'s>(&self, source: &'s str) -> &'s str {
        self.span.bytes().map_or("", |bytes| &source[bytes])
    }
}

impl Token for NumericLiteral {
    fn kind(&self) -> TokenKind {
        TokenKind::NumericLiteral
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        self.after_space
    }

    fn value<'s>(&self, source: &'s str) -> Cow<'s, str> {
        Cow::Owned(self.number(source).to_string())
    }

    fn describe(&self, source: &str) -> String {
        format!("Number({})", self.raw(source))
    }

    fn display_value<'s>(&self, source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed(self.raw(source))
    }

    fn as_number(&self) -> Option<&NumericLiteral> {
        Some(self)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NumberShape {
    previous: u8,
    point: bool,
    exponent: bool,
    exponent_digits: bool,
    malformed: bool,
}

impl NumberShape {
    pub(crate) fn take(&mut self, byte: u8) -> bool {
        if byte.is_ascii_digit() {
            self.exponent_digits |= self.exponent;
        } else if byte == NumericLiteral::DECIMAL_POINT {
            self.malformed |= self.point || self.exponent;
            self.point = true;
        } else if NumericLiteral::EXPONENT_MARKERS.contains(&byte) {
            self.malformed |= self.exponent;
            self.exponent = true;
        } else if !(NumericLiteral::EXPONENT_SIGNS.contains(&byte)
            && NumericLiteral::EXPONENT_MARKERS.contains(&self.previous))
        {
            return false;
        }

        self.previous = byte;

        true
    }

    pub(crate) fn is_valid(&self) -> bool {
        !self.malformed && (!self.exponent || self.exponent_digits)
    }
}

impl<'src> Lexer<'src> {
    pub(crate) fn scan_number(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
    ) -> LexResult {
        let mut shape = NumberShape::default();

        while !self.is_at_end(stream) && shape.take(self.code_at(stream, 0)) {
            stream.cursor.advance(1);
        }

        if shape.is_valid() {
            return Ok(
                token_start.finish(stream, |span, after_space| NumericLiteral {
                    span,
                    after_space,
                }),
            );
        }

        let raw = token_start.text(self, stream);

        self.raise_at(
            stream,
            token_start,
            LexerDiagnostic::InvalidNumericLiteral(raw.to_owned()),
        )
    }
}

#[cfg(test)]
#[path = "tests/number.rs"]
mod tests;
