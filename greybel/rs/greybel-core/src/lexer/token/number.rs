use std::borrow::Cow;

use super::{PendingToken, Token, TokenKind};
use crate::diagnostics::LexerDiagnostic;
use crate::dialect::SyntaxFeatures;
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
    pub const BINARY_PREFIXES: [&str; 2] = ["0b", "0B"];
    pub const DIGIT_SEPARATOR: u8 = b'_';
    pub const HEX_PREFIXES: [&str; 2] = ["0x", "0X"];
    pub const OCTAL_PREFIXES: [&str; 2] = ["0o", "0O"];

    #[must_use]
    pub fn number(&self, source: &str) -> f64 {
        Self::value_of(self.raw(source))
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn value_of(raw: &str) -> f64 {
        let radix_digits =
            |prefixes: [&str; 2]| prefixes.iter().find_map(|prefix| raw.strip_prefix(prefix));
        let digits = |text: &str| text.replace(char::from(Self::DIGIT_SEPARATOR), "");

        if let Some(hex) = radix_digits(Self::HEX_PREFIXES) {
            return u128::from_str_radix(&digits(hex), 16)
                .map_or(f64::INFINITY, |value| value as f64);
        }

        if let Some(binary) = radix_digits(Self::BINARY_PREFIXES) {
            return u128::from_str_radix(&digits(binary), 2)
                .map_or(f64::INFINITY, |value| value as f64);
        }

        if let Some(octal) = radix_digits(Self::OCTAL_PREFIXES) {
            return u128::from_str_radix(&digits(octal), 8)
                .map_or(f64::INFINITY, |value| value as f64);
        }

        digits(raw)
            .parse()
            .expect("the lexer only accepts numbers in the shape `str::parse` reads")
    }

    #[must_use]
    pub fn is_plain(raw: &str) -> bool {
        !raw.bytes().any(|byte| {
            matches!(
                byte,
                b'x' | b'X' | b'b' | b'B' | b'o' | b'O' | Self::DIGIT_SEPARATOR
            )
        })
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
    separators: bool,
}

impl NumberShape {
    fn with_separators() -> Self {
        Self {
            separators: true,
            ..Self::default()
        }
    }

    pub(crate) fn take(&mut self, byte: u8) -> bool {
        if byte.is_ascii_digit() {
            self.exponent_digits |= self.exponent;
        } else if byte == NumericLiteral::DIGIT_SEPARATOR && self.separators {
            self.malformed |= !self.previous.is_ascii_digit();
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

    pub(crate) fn is_valid(self) -> bool {
        !self.malformed
            && (!self.exponent || self.exponent_digits)
            && self.previous != NumericLiteral::DIGIT_SEPARATOR
    }
}

impl<'src> Lexer<'src> {
    pub(crate) fn scan_number(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
    ) -> LexResult {
        let extensions = self.grammar().allows(SyntaxFeatures::NUMBER_LITERALS);

        if extensions && let Some(radix) = self.radix_prefix(stream) {
            return self.scan_radix_number(stream, token_start, radix);
        }

        let mut shape = if extensions {
            NumberShape::with_separators()
        } else {
            NumberShape::default()
        };

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

impl Lexer<'_> {
    fn radix_prefix(&self, stream: &TokenStream) -> Option<u32> {
        if self.code_at(stream, 0) != b'0' {
            return None;
        }

        match self.code_at(stream, 1) {
            b'x' | b'X' => Some(16),
            b'b' | b'B' => Some(2),
            b'o' | b'O' => Some(8),
            _ => None,
        }
    }

    fn scan_radix_number(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
        radix: u32,
    ) -> LexResult {
        stream.cursor.advance(2);

        let mut digits = 0;
        let mut previous = b'x';
        let mut malformed = false;

        while !self.is_at_end(stream) {
            let byte = self.code_at(stream, 0);

            if char::from(byte).is_digit(radix) {
                digits += 1;
            } else if byte == NumericLiteral::DIGIT_SEPARATOR {
                malformed |= !char::from(previous).is_digit(radix);
            } else if byte.is_ascii_alphanumeric() {
                malformed = true;
            } else {
                break;
            }

            previous = byte;
            stream.cursor.advance(1);
        }

        if digits > 0 && !malformed && previous != NumericLiteral::DIGIT_SEPARATOR {
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
