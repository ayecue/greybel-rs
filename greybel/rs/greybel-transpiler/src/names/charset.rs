use std::ops::RangeInclusive;

const UPPERCASE: RangeInclusive<char> = 'A'..='Z';
const LOWERCASE: RangeInclusive<char> = 'a'..='z';
const DIGITS: RangeInclusive<char> = '0'..='9';
const SEPARATOR: char = '_';
const OBFUSCATION_CODES: RangeInclusive<u32> = 1000..=2000;

fn suffix_chars(obfuscation: bool) -> impl Iterator<Item = char> {
    let obfuscation_chars = obfuscation
        .then_some(OBFUSCATION_CODES)
        .into_iter()
        .flatten()
        .filter_map(char::from_u32);

    std::iter::once(SEPARATOR).chain(obfuscation_chars)
}

#[must_use]
pub fn variable_charset(obfuscation: bool) -> Vec<char> {
    UPPERCASE
        .chain(LOWERCASE)
        .chain(suffix_chars(obfuscation))
        .collect()
}

#[must_use]
pub fn module_charset(obfuscation: bool) -> Vec<char> {
    UPPERCASE
        .chain(LOWERCASE)
        .chain(DIGITS)
        .chain(suffix_chars(obfuscation))
        .collect()
}
