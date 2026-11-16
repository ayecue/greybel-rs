use greybel_symbols::delimiters::{QUOTE, SLICE_SEPARATOR, STATEMENT_SEPARATOR};

use super::chars::{CARRIAGE_RETURN, LINE_FEED, is_identifier_start};
use super::{NumericLiteral, Punct};
use crate::dialect::{LexerRuleTable, SyntaxFeatures};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenStart {
    Invalid,
    LineBreak,
    Identifier,
    Quote,
    Digit,
    Dot,
    Colon,
    Slash,
    Punct,
    Hash,
}

#[derive(Debug, Clone)]
pub(crate) struct TokenStartTable(Box<[TokenStart; 256]>);

impl TokenStartTable {
    pub(crate) fn new(
        rules: LexerRuleTable,
        features: SyntaxFeatures,
        punctuators: impl IntoIterator<Item = Punct>,
    ) -> Self {
        let mut starts = Box::new([TokenStart::Invalid; 256]);

        for punct in punctuators {
            starts[usize::from(punct.as_str().as_bytes()[0])] = TokenStart::Punct;
        }

        for byte in 0..=u8::MAX {
            if is_identifier_start(byte) {
                starts[usize::from(byte)] = TokenStart::Identifier;
            }
        }

        if rules.hash_identifiers {
            starts[usize::from(b'#')] = TokenStart::Identifier;
        }

        if features.contains(SyntaxFeatures::TEMPLATES) {
            starts[usize::from(b'#')] = TokenStart::Hash;
        }

        for digit in b'0'..=b'9' {
            starts[usize::from(digit)] = TokenStart::Digit;
        }

        let fixed_entries = [
            (QUOTE.as_bytes()[0], TokenStart::Quote),
            (NumericLiteral::DECIMAL_POINT, TokenStart::Dot),
            (SLICE_SEPARATOR.as_bytes()[0], TokenStart::Colon),
            (b'/', TokenStart::Slash),
            (LINE_FEED, TokenStart::LineBreak),
            (CARRIAGE_RETURN, TokenStart::LineBreak),
            (STATEMENT_SEPARATOR.as_bytes()[0], TokenStart::LineBreak),
        ];

        for (byte, start) in fixed_entries {
            starts[usize::from(byte)] = start;
        }

        Self(starts)
    }

    pub(crate) fn get(&self, byte: u8) -> TokenStart {
        self.0[usize::from(byte)]
    }
}
