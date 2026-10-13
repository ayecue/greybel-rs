use bitflags::bitflags;

use crate::buffer::{MAX_DIGITS, NumberBuffer};
use crate::info::{INVARIANT, NumberFormatInfo};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct NumberStyles: u32 {
        const ALLOW_LEADING_WHITE = 0x001;
        const ALLOW_TRAILING_WHITE = 0x002;
        const ALLOW_LEADING_SIGN = 0x004;
        const ALLOW_TRAILING_SIGN = 0x008;
        const ALLOW_PARENTHESES = 0x010;
        const ALLOW_DECIMAL_POINT = 0x020;
        const ALLOW_THOUSANDS = 0x040;
        const ALLOW_EXPONENT = 0x080;
        const ALLOW_CURRENCY_SYMBOL = 0x100;
        const ALLOW_HEX_SPECIFIER = 0x200;

        const INTEGER = Self::ALLOW_LEADING_WHITE.bits()
            | Self::ALLOW_TRAILING_WHITE.bits()
            | Self::ALLOW_LEADING_SIGN.bits();
        const HEX_NUMBER = Self::ALLOW_LEADING_WHITE.bits()
            | Self::ALLOW_TRAILING_WHITE.bits()
            | Self::ALLOW_HEX_SPECIFIER.bits();
        const NUMBER = Self::INTEGER.bits()
            | Self::ALLOW_TRAILING_SIGN.bits()
            | Self::ALLOW_DECIMAL_POINT.bits()
            | Self::ALLOW_THOUSANDS.bits();
        const FLOAT = Self::INTEGER.bits()
            | Self::ALLOW_DECIMAL_POINT.bits()
            | Self::ALLOW_EXPONENT.bits();
        const CURRENCY = Self::NUMBER.bits()
            | Self::ALLOW_PARENTHESES.bits()
            | Self::ALLOW_CURRENCY_SYMBOL.bits();
        const ANY = Self::CURRENCY.bits() | Self::ALLOW_EXPONENT.bits();
    }
}

const STATE_SIGN: u32 = 0x01;
const STATE_PARENS: u32 = 0x02;
const STATE_DIGITS: u32 = 0x04;
const STATE_NON_ZERO: u32 = 0x08;
const STATE_DECIMAL: u32 = 0x10;
const STATE_CURRENCY: u32 = 0x20;

const MAX_EXPONENT: i32 = 1000;
const OVERFLOWED_EXPONENT: i32 = 9999;
const INT32_PRECISION: i32 = 10;

#[must_use]
pub fn parse_f64(text: &str, styles: NumberStyles) -> Option<f64> {
    assert!(
        !styles.contains(NumberStyles::ALLOW_HEX_SPECIFIER),
        "AllowHexSpecifier is not supported on floating point data types"
    );

    let info = &INVARIANT;

    if let Some(number) = string_to_number(text, styles, info, false) {
        return number_buffer_to_f64(&number);
    }

    let unpadded_text = text.trim();

    if unpadded_text == info.positive_infinity_symbol {
        Some(f64::INFINITY)
    } else if unpadded_text == info.negative_infinity_symbol {
        Some(f64::NEG_INFINITY)
    } else if unpadded_text == info.nan_symbol {
        Some(f64::NAN)
    } else {
        None
    }
}

#[must_use]
pub fn parse_i32(text: &str, styles: NumberStyles) -> Option<i32> {
    let info = &INVARIANT;

    if NumberStyles::INTEGER.contains(styles) {
        return parse_i32_integer_style(text.as_bytes(), styles);
    }

    if styles.contains(NumberStyles::ALLOW_HEX_SPECIFIER) {
        assert!(
            NumberStyles::HEX_NUMBER.contains(styles),
            "AllowHexSpecifier combines only with HexNumber's flags"
        );

        return parse_u32_hex_style(text.as_bytes(), styles).map(|value| value as i32);
    }

    string_to_number(text, styles, info, false).and_then(|number| number_to_i32(&number))
}

fn number_buffer_to_f64(number: &NumberBuffer) -> Option<f64> {
    let value = number.to_f64();

    if !value.is_finite() {
        return None;
    }

    Some(if value == 0.0 { 0.0 } else { value })
}

fn string_to_number(
    text: &str,
    styles: NumberStyles,
    info: &NumberFormatInfo,
    parse_decimal: bool,
) -> Option<NumberBuffer> {
    let (number, end) = parse_number(text, styles, info, parse_decimal)?;

    if end < text.len() && !trailing_zeros(&text.as_bytes()[end..]) {
        return None;
    }

    Some(number)
}

fn trailing_zeros(remainder: &[u8]) -> bool {
    remainder.iter().all(|&byte| byte == 0)
}

fn is_white(c: char) -> bool {
    matches!(c, ' ' | '\t'..='\r')
}

fn char_at(text: &str, at: usize) -> char {
    text.get(at..)
        .and_then(|remainder| remainder.chars().next())
        .unwrap_or('\0')
}

fn match_chars(text: &str, mut at: usize, value: &str) -> Option<usize> {
    if value.is_empty() {
        return None;
    }

    for target_char in value.chars() {
        let actual = char_at(text, at);

        if actual != target_char && !(target_char == '\u{a0}' && actual == ' ') {
            return None;
        }

        at += actual.len_utf8();
    }

    Some(at)
}

#[allow(clippy::too_many_lines)]
fn parse_number(
    text: &str,
    styles: NumberStyles,
    info: &NumberFormatInfo,
    parse_decimal: bool,
) -> Option<(NumberBuffer, usize)> {
    let mut number = NumberBuffer::new();
    let parsing_currency = styles.contains(NumberStyles::ALLOW_CURRENCY_SYMBOL);
    let (decimal_separator, group_separator) = if parsing_currency {
        (
            info.currency_decimal_separator,
            info.currency_group_separator,
        )
    } else {
        (info.number_decimal_separator, info.number_group_separator)
    };
    let mut currency_symbol = parsing_currency.then_some(info.currency_symbol);

    let mut state = 0;
    let mut at = 0;
    let mut ch = char_at(text, at);
    let sign = |number: &mut NumberBuffer, at: usize| {
        match_chars(text, at, info.positive_sign).or_else(|| {
            let next = match_chars(text, at, info.negative_sign);

            if next.is_some() {
                number.sign = true;
            }

            next
        })
    };

    loop {
        let blocked_white = !is_white(ch)
            || !styles.contains(NumberStyles::ALLOW_LEADING_WHITE)
            || (state & STATE_SIGN != 0
                && state & STATE_CURRENCY == 0
                && info.number_negative_pattern != 2);

        if blocked_white {
            let next = if styles.contains(NumberStyles::ALLOW_LEADING_SIGN)
                && state & STATE_SIGN == 0
                && let Some(next) = sign(&mut number, at)
            {
                state |= STATE_SIGN;

                next
            } else if ch == '('
                && styles.contains(NumberStyles::ALLOW_PARENTHESES)
                && state & STATE_SIGN == 0
            {
                state |= STATE_SIGN | STATE_PARENS;
                number.sign = true;

                at + 1
            } else if let Some(symbol) = currency_symbol
                && let Some(next) = match_chars(text, at, symbol)
            {
                state |= STATE_CURRENCY;
                currency_symbol = None;

                next
            } else {
                break;
            };

            at = next;
        } else {
            at += ch.len_utf8();
        }

        ch = char_at(text, at);
    }

    let mut digit_count = 0;
    let mut digit_end = 0;

    loop {
        if ch.is_ascii_digit() {
            state |= STATE_DIGITS;
            if ch != '0' || state & STATE_NON_ZERO != 0 {
                if digit_count < MAX_DIGITS {
                    number.set_digit(digit_count, ch as u8);
                    digit_count += 1;
                    if ch != '0' || parse_decimal {
                        digit_end = digit_count;
                    }
                }

                if state & STATE_DECIMAL == 0 {
                    number.scale += 1;
                }

                state |= STATE_NON_ZERO;
            } else if state & STATE_DECIMAL != 0 {
                number.scale -= 1;
            }

            at += 1;
        } else if styles.contains(NumberStyles::ALLOW_DECIMAL_POINT)
            && state & STATE_DECIMAL == 0
            && let Some(next) = match_chars(text, at, decimal_separator).or_else(|| {
                (parsing_currency && state & STATE_CURRENCY == 0)
                    .then(|| match_chars(text, at, info.number_decimal_separator))
                    .flatten()
            })
        {
            state |= STATE_DECIMAL;
            at = next;
        } else if styles.contains(NumberStyles::ALLOW_THOUSANDS)
            && state & STATE_DIGITS != 0
            && state & STATE_DECIMAL == 0
            && let Some(next) = match_chars(text, at, group_separator).or_else(|| {
                (parsing_currency && state & STATE_CURRENCY == 0)
                    .then(|| match_chars(text, at, info.number_group_separator))
                    .flatten()
            })
        {
            at = next;
        } else {
            break;
        }

        ch = char_at(text, at);
    }

    number.precision = i32::try_from(digit_end).expect("at most 50 digits");
    number.set_digit(digit_end, 0);
    if state & STATE_DIGITS == 0 {
        return None;
    }

    if matches!(ch, 'E' | 'e') && styles.contains(NumberStyles::ALLOW_EXPONENT) {
        let before = at;

        at += 1;

        let mut negative = false;

        if let Some(next) = match_chars(text, at, info.positive_sign) {
            at = next;
        } else if let Some(next) = match_chars(text, at, info.negative_sign) {
            at = next;
            negative = true;
        }

        ch = char_at(text, at);

        if ch.is_ascii_digit() {
            let mut exponent: i32 = 0;

            loop {
                exponent = exponent * 10 + (ch as i32 - '0' as i32);
                at += 1;
                ch = char_at(text, at);
                if exponent > MAX_EXPONENT {
                    exponent = OVERFLOWED_EXPONENT;
                    while ch.is_ascii_digit() {
                        at += 1;
                        ch = char_at(text, at);
                    }
                }

                if !ch.is_ascii_digit() {
                    break;
                }
            }

            number.scale += if negative { -exponent } else { exponent };
        } else {
            at = before;
            ch = char_at(text, at);
        }
    }

    loop {
        if !is_white(ch) || !styles.contains(NumberStyles::ALLOW_TRAILING_WHITE) {
            let next = if styles.contains(NumberStyles::ALLOW_TRAILING_SIGN)
                && state & STATE_SIGN == 0
                && let Some(next) = sign(&mut number, at)
            {
                state |= STATE_SIGN;

                next
            } else if ch == ')' && state & STATE_PARENS != 0 {
                state &= !STATE_PARENS;

                at + 1
            } else if let Some(symbol) = currency_symbol
                && let Some(next) = match_chars(text, at, symbol)
            {
                currency_symbol = None;

                next
            } else {
                break;
            };

            at = next;
        } else {
            at += ch.len_utf8();
        }

        ch = char_at(text, at);
    }

    if state & STATE_PARENS != 0 {
        return None;
    }

    if state & STATE_NON_ZERO == 0 {
        if !parse_decimal {
            number.scale = 0;
        }

        if state & STATE_DECIMAL == 0 {
            number.sign = false;
        }
    }

    Some((number, at))
}

fn number_to_i32(number: &NumberBuffer) -> Option<i32> {
    let mut scale_left = number.scale;

    if scale_left > INT32_PRECISION || scale_left < number.precision {
        return None;
    }

    let mut value: i32 = 0;
    let mut cursor = 0;

    while scale_left > 0 {
        scale_left -= 1;
        if value as u32 > i32::MAX as u32 / 10 {
            return None;
        }

        value = value.wrapping_mul(10);

        let digit = number.digit(cursor);

        if digit != 0 {
            value = value.wrapping_add(i32::from(digit - b'0'));
            cursor += 1;
        }
    }

    if number.sign {
        value = value.wrapping_neg();

        (value <= 0).then_some(value)
    } else {
        (value >= 0).then_some(value)
    }
}

fn is_white_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t'..=b'\r')
}

enum DigitScan {
    End {
        value: u32,
        overflow: bool,
    },
    Trailing {
        value: u32,
        overflow: bool,
        at: usize,
    },
}

fn finish_digits(text: &[u8], styles: NumberStyles, digits: DigitScan) -> Option<u32> {
    let (value, overflow) = match digits {
        DigitScan::End { value, overflow } => (value, overflow),
        DigitScan::Trailing {
            value,
            overflow,
            mut at,
        } => {
            if is_white_byte(text[at]) {
                if !styles.contains(NumberStyles::ALLOW_TRAILING_WHITE) {
                    return None;
                }

                at += 1;
                while at < text.len() && is_white_byte(text[at]) {
                    at += 1;
                }
            }

            if !trailing_zeros(&text[at..]) {
                return None;
            }

            (value, overflow)
        }
    };

    (!overflow).then_some(value)
}

fn parse_i32_integer_style(text: &[u8], styles: NumberStyles) -> Option<i32> {
    let mut at = 0;
    let mut current = *text.first()?;
    let advance = |at: &mut usize| -> Option<u8> {
        *at += 1;

        text.get(*at).copied()
    };

    if styles.contains(NumberStyles::ALLOW_LEADING_WHITE) && is_white_byte(current) {
        loop {
            current = advance(&mut at)?;
            if !is_white_byte(current) {
                break;
            }
        }
    }

    let mut negative = false;

    if styles.contains(NumberStyles::ALLOW_LEADING_SIGN) {
        if current == b'-' {
            negative = true;
            current = advance(&mut at)?;
        } else if current == b'+' {
            current = advance(&mut at)?;
        }
    }

    if !current.is_ascii_digit() {
        return None;
    }

    let digits = 'digits: {
        let mut value: u32 = 0;
        let mut overflow = false;

        if current == b'0' {
            loop {
                let Some(next) = advance(&mut at) else {
                    break 'digits DigitScan::End { value, overflow };
                };

                current = next;
                if current != b'0' {
                    break;
                }
            }

            if !current.is_ascii_digit() {
                break 'digits DigitScan::Trailing {
                    value,
                    overflow,
                    at,
                };
            }
        }

        value = u32::from(current - b'0');
        for _ in 0..8 {
            let Some(next) = advance(&mut at) else {
                break 'digits DigitScan::End { value, overflow };
            };

            current = next;
            if !current.is_ascii_digit() {
                break 'digits DigitScan::Trailing {
                    value,
                    overflow,
                    at,
                };
            }

            value = 10 * value + u32::from(current - b'0');
        }

        let Some(next) = advance(&mut at) else {
            break 'digits DigitScan::End { value, overflow };
        };

        current = next;
        if !current.is_ascii_digit() {
            break 'digits DigitScan::Trailing {
                value,
                overflow,
                at,
            };
        }

        if value > i32::MAX as u32 / 10 {
            overflow = true;
        }

        value = value
            .wrapping_mul(10)
            .wrapping_add(u32::from(current - b'0'));
        if value > i32::MAX as u32 + u32::from(negative) {
            overflow = true;
        }

        loop {
            let Some(next) = advance(&mut at) else {
                break 'digits DigitScan::End { value, overflow };
            };

            current = next;
            if !current.is_ascii_digit() {
                break 'digits DigitScan::Trailing {
                    value,
                    overflow,
                    at,
                };
            }

            overflow = true;
        }
    };
    let value = finish_digits(text, styles, digits)? as i32;

    Some(if negative {
        value.wrapping_neg()
    } else {
        value
    })
}

fn parse_u32_hex_style(text: &[u8], styles: NumberStyles) -> Option<u32> {
    let hex = |byte: u8| char::from(byte).to_digit(16);
    let mut at = 0;
    let mut current = *text.first()?;
    let advance = |at: &mut usize| -> Option<u8> {
        *at += 1;

        text.get(*at).copied()
    };

    if styles.contains(NumberStyles::ALLOW_LEADING_WHITE) && is_white_byte(current) {
        loop {
            current = advance(&mut at)?;
            if !is_white_byte(current) {
                break;
            }
        }
    }

    let mut value = hex(current)?;
    let digits = 'digits: {
        let overflow = false;

        if current == b'0' {
            loop {
                let Some(next) = advance(&mut at) else {
                    break 'digits DigitScan::End { value: 0, overflow };
                };

                current = next;
                if current != b'0' {
                    break;
                }
            }

            match hex(current) {
                Some(digit) => value = digit,
                None => {
                    break 'digits DigitScan::Trailing {
                        value: 0,
                        overflow,
                        at,
                    };
                }
            }
        }

        for _ in 0..7 {
            let Some(next) = advance(&mut at) else {
                break 'digits DigitScan::End { value, overflow };
            };

            current = next;

            let Some(digit) = hex(current) else {
                break 'digits DigitScan::Trailing {
                    value,
                    overflow,
                    at,
                };
            };

            value = 16 * value + digit;
        }

        let Some(next) = advance(&mut at) else {
            break 'digits DigitScan::End { value, overflow };
        };

        current = next;
        if hex(current).is_none() {
            break 'digits DigitScan::Trailing {
                value,
                overflow,
                at,
            };
        }

        loop {
            let Some(next) = advance(&mut at) else {
                break 'digits DigitScan::End {
                    value,
                    overflow: true,
                };
            };

            current = next;
            if hex(current).is_none() {
                break 'digits DigitScan::Trailing {
                    value,
                    overflow: true,
                    at,
                };
            }
        }
    };

    finish_digits(text, styles, digits)
}
