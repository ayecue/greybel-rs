use std::ops::RangeInclusive;

const UPPERCASE: RangeInclusive<char> = 'A'..='Z';
const LOWERCASE: RangeInclusive<char> = 'a'..='z';
const DIGITS: RangeInclusive<char> = '0'..='9';
const SEPARATOR: char = '_';
const OBFUSCATION_CODES: RangeInclusive<u32> = 1000..=2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Charset {
    digits: bool,
    obfuscation: bool,
}

impl Charset {
    #[must_use]
    pub fn variables(obfuscation: bool) -> Self {
        Self {
            digits: false,
            obfuscation,
        }
    }

    #[must_use]
    pub fn modules(obfuscation: bool) -> Self {
        Self {
            digits: true,
            obfuscation,
        }
    }

    #[must_use]
    pub fn chars(self) -> Vec<char> {
        let digits = self.digits.then_some(DIGITS).into_iter().flatten();

        UPPERCASE
            .chain(LOWERCASE)
            .chain(digits)
            .chain(self.suffix_chars())
            .collect()
    }

    fn suffix_chars(self) -> impl Iterator<Item = char> {
        let obfuscation_chars = self
            .obfuscation
            .then_some(OBFUSCATION_CODES)
            .into_iter()
            .flatten()
            .filter_map(char::from_u32);

        std::iter::once(SEPARATOR).chain(obfuscation_chars)
    }
}
