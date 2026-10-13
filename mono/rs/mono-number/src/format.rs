use std::fmt::Write;

use thiserror::Error;

use crate::buffer::NumberBuffer;
use crate::info::{INVARIANT, NumberFormatInfo};

const FLOAT_PRECISION: i32 = 7;
const DOUBLE_PRECISION: i32 = 15;
const FLOAT_ROUND_TRIP_PRECISION: i32 = 9;
const DOUBLE_ROUND_TRIP_PRECISION: i32 = 17;

const POSITIVE_NUMBER_FORMAT: &str = "#";
const NEGATIVE_NUMBER_FORMATS: [&str; 5] = ["(#)", "-#", "- #", "#-", "# -"];
const POSITIVE_CURRENCY_FORMATS: [&str; 4] = ["$#", "#$", "$ #", "# $"];
const NEGATIVE_CURRENCY_FORMATS: [&str; 16] = [
    "($#)", "-$#", "$-#", "$#-", "(#$)", "-#$", "#-$", "#$-", "-# $", "-$ #", "# $-", "$ #-",
    "$ -#", "#- $", "($ #)", "(# $)",
];
const POSITIVE_PERCENT_FORMATS: [&str; 4] = ["# %", "#%", "%#", "% #"];
const NEGATIVE_PERCENT_FORMATS: [&str; 12] = [
    "-# %", "-#%", "-%#", "%-#", "%#-", "#-%", "#%-", "-% #", "# %-", "% #-", "% -#", "#- %",
];

const NO_DIGIT: i32 = 0x7FFF_FFFF;
const MAX_EXPONENT_DIGITS: i32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("Format specifier was invalid.")]
pub struct FormatError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    specifier: char,
    digits: i32,
    custom: Box<[char]>,
}

impl Format {
    #[must_use]
    pub fn new(format: &str) -> Self {
        let chars: Vec<char> = format.chars().collect();
        let (specifier, digits) = parse_specifier(&chars);
        let custom = if specifier == '\0' {
            chars.into_boxed_slice()
        } else {
            Box::default()
        };

        Self {
            specifier,
            digits,
            custom,
        }
    }

    pub fn f64(&self, value: f64) -> Result<String, FormatError> {
        let mut out = String::new();

        self.write_f64(&mut out, value)?;

        Ok(out)
    }

    pub fn f32(&self, value: f32) -> Result<String, FormatError> {
        let mut out = String::new();

        self.write_f32(&mut out, value)?;

        Ok(out)
    }

    pub fn write_f64(&self, out: &mut String, value: f64) -> Result<(), FormatError> {
        let info = &INVARIANT;
        let mut precision = DOUBLE_PRECISION;

        match self.specifier {
            'R' | 'r' => {
                let mut number = NumberBuffer::from_f64(value, DOUBLE_PRECISION);

                if let Some(symbol) = special_symbol(&number, info) {
                    out.push_str(symbol);
                } else if number.to_f64() == value {
                    number_to_string(out, &mut number, 'G', DOUBLE_PRECISION, info)?;
                } else {
                    let mut number = NumberBuffer::from_f64(value, DOUBLE_ROUND_TRIP_PRECISION);

                    number_to_string(out, &mut number, 'G', DOUBLE_ROUND_TRIP_PRECISION, info)?;
                }

                return Ok(());
            }
            'E' | 'e' if self.digits > 14 => precision = DOUBLE_ROUND_TRIP_PRECISION,
            'G' | 'g' if self.digits > 15 => precision = DOUBLE_ROUND_TRIP_PRECISION,
            _ => {}
        }

        let mut number = NumberBuffer::from_f64(value, precision);

        self.write(out, &mut number, info)
    }

    pub fn write_f32(&self, out: &mut String, value: f32) -> Result<(), FormatError> {
        let info = &INVARIANT;
        let mut precision = FLOAT_PRECISION;

        match self.specifier {
            'R' | 'r' => {
                let mut number = NumberBuffer::from_f64(f64::from(value), FLOAT_PRECISION);

                if let Some(symbol) = special_symbol(&number, info) {
                    out.push_str(symbol);
                } else if number.to_f64() as f32 == value {
                    number_to_string(out, &mut number, 'G', FLOAT_PRECISION, info)?;
                } else {
                    let mut number =
                        NumberBuffer::from_f64(f64::from(value), FLOAT_ROUND_TRIP_PRECISION);

                    number_to_string(out, &mut number, 'G', FLOAT_ROUND_TRIP_PRECISION, info)?;
                }

                return Ok(());
            }
            'E' | 'e' if self.digits > 6 => precision = FLOAT_ROUND_TRIP_PRECISION,
            'G' | 'g' if self.digits > 7 => precision = FLOAT_ROUND_TRIP_PRECISION,
            _ => {}
        }

        let mut number = NumberBuffer::from_f64(f64::from(value), precision);

        self.write(out, &mut number, info)
    }

    fn write(
        &self,
        out: &mut String,
        number: &mut NumberBuffer,
        info: &NumberFormatInfo,
    ) -> Result<(), FormatError> {
        if let Some(symbol) = special_symbol(number, info) {
            out.push_str(symbol);
        } else if self.specifier == '\0' {
            number_to_string_format(out, number, &self.custom, info);
        } else {
            number_to_string(out, number, self.specifier, self.digits, info)?;
        }

        Ok(())
    }
}

pub fn format_f64(value: f64, format: &str) -> Result<String, FormatError> {
    Format::new(format).f64(value)
}

pub fn format_f32(value: f32, format: &str) -> Result<String, FormatError> {
    Format::new(format).f32(value)
}

fn special_symbol(number: &NumberBuffer, info: &NumberFormatInfo) -> Option<&'static str> {
    if number.is_nan() {
        Some(info.nan_symbol)
    } else if number.is_infinity() {
        Some(if number.sign {
            info.negative_infinity_symbol
        } else {
            info.positive_infinity_symbol
        })
    } else {
        None
    }
}

fn parse_specifier(format: &[char]) -> (char, i32) {
    if let Some(&c) = format.first()
        && c.is_ascii_alphabetic()
    {
        let digit = |index: usize| format[index].to_digit(10).map(|digit| digit as i32);

        match format.len() {
            1 => return (c, -1),
            2 => {
                if let Some(digit) = digit(1) {
                    return (c, digit);
                }
            }
            3 => {
                if let (Some(tens), Some(ones)) = (digit(1), digit(2)) {
                    return (c, tens * 10 + ones);
                }
            }
            _ => {}
        }

        let mut n = 0;
        let mut index = 1;

        while index < format.len() && n < 10 {
            let Some(digit) = digit(index) else { break };

            n = n * 10 + digit;
            index += 1;
        }

        if index == format.len() || format[index] == '\0' {
            return (c, n);
        }
    }

    let standard = format.first().is_none_or(|&c| c == '\0');

    (if standard { 'G' } else { '\0' }, -1)
}

fn push_digit(out: &mut String, number: &NumberBuffer, cursor: &mut usize) {
    out.push(next_digit(number, cursor).unwrap_or('0'));
}

fn next_digit(number: &NumberBuffer, cursor: &mut usize) -> Option<char> {
    let digit = number.digit(*cursor);

    (digit != 0).then(|| {
        *cursor += 1;

        char::from(digit)
    })
}

fn number_to_string(
    out: &mut String,
    number: &mut NumberBuffer,
    format: char,
    mut max_digits: i32,
    info: &NumberFormatInfo,
) -> Result<(), FormatError> {
    let min_digits;

    match format {
        'C' | 'c' => {
            min_digits = if max_digits >= 0 {
                max_digits
            } else {
                info.currency_decimal_digits
            };
            if max_digits < 0 {
                max_digits = info.currency_decimal_digits;
            }

            round_number(number, number.scale + max_digits);
            format_currency(out, number, min_digits, max_digits, info);
        }
        'F' | 'f' => {
            if max_digits < 0 {
                max_digits = info.number_decimal_digits;
            }

            min_digits = max_digits;
            round_number(number, number.scale + max_digits);
            if number.sign {
                out.push_str(info.negative_sign);
            }

            format_fixed_point(
                out,
                number,
                min_digits,
                max_digits,
                None,
                info.number_decimal_separator,
                "",
            );
        }
        'N' | 'n' => {
            if max_digits < 0 {
                max_digits = info.number_decimal_digits;
            }

            min_digits = max_digits;
            round_number(number, number.scale + max_digits);
            format_number(out, number, min_digits, max_digits, info);
        }
        'E' | 'e' => {
            if max_digits < 0 {
                max_digits = 6;
            }

            min_digits = max_digits;
            max_digits += 1;
            round_number(number, max_digits);
            if number.sign {
                out.push_str(info.negative_sign);
            }

            format_scientific(out, number, min_digits, max_digits, info, format);
        }
        'G' | 'g' => {
            if max_digits < 1 {
                max_digits = number.precision;
            }

            min_digits = max_digits;
            round_number(number, max_digits);
            if number.sign {
                out.push_str(info.negative_sign);
            }

            let exponent = if format == 'G' { 'E' } else { 'e' };

            format_general(out, number, min_digits, max_digits, info, exponent, false);
        }
        'P' | 'p' => {
            if max_digits < 0 {
                max_digits = info.percent_decimal_digits;
            }

            min_digits = max_digits;
            number.scale += 2;
            round_number(number, number.scale + max_digits);
            format_percent(out, number, min_digits, max_digits, info);
        }
        _ => return Err(FormatError),
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn number_to_string_format(
    out: &mut String,
    number: &mut NumberBuffer,
    format: &[char],
    info: &NumberFormatInfo,
) {
    let len = format.len();
    let mut digit_count;
    let mut decimal_pos;
    let mut first_digit;
    let mut last_digit;
    let mut scientific;
    let mut thousand_pos;
    let mut thousand_count = 0;
    let mut thousand_seps;
    let mut scale_adjust;

    let initial = if number.digit(0) == 0 {
        2
    } else if number.sign {
        1
    } else {
        0
    };
    let mut section = find_section(format, initial);

    loop {
        digit_count = 0;
        decimal_pos = -1;
        first_digit = NO_DIGIT;
        last_digit = 0;
        scientific = false;
        thousand_pos = -1;
        thousand_seps = false;
        scale_adjust = 0;

        let mut src = section;

        while src < len {
            let ch = format[src];

            src += 1;
            if ch == '\0' || ch == ';' {
                break;
            }

            match ch {
                '#' => digit_count += 1,
                '0' => {
                    if first_digit == NO_DIGIT {
                        first_digit = digit_count;
                    }

                    digit_count += 1;
                    last_digit = digit_count;
                }
                '.' => {
                    if decimal_pos < 0 {
                        decimal_pos = digit_count;
                    }
                }
                ',' => {
                    if digit_count > 0 && decimal_pos < 0 {
                        if thousand_pos >= 0 {
                            if thousand_pos == digit_count {
                                thousand_count += 1;
                                continue;
                            }

                            thousand_seps = true;
                        }

                        thousand_pos = digit_count;
                        thousand_count = 1;
                    }
                }
                '%' => scale_adjust += 2,
                '\u{2030}' => scale_adjust += 3,
                '\'' | '"' => skip_quote_span(format, &mut src, ch),
                '\\' => {
                    if src < len && format[src] != '\0' {
                        src += 1;
                    }
                }
                'E' | 'e' => {
                    let at = |index: usize| format.get(index).copied();

                    if at(src) == Some('0')
                        || (matches!(at(src), Some('+' | '-')) && at(src + 1) == Some('0'))
                    {
                        src += 1;
                        while at(src) == Some('0') {
                            src += 1;
                        }

                        scientific = true;
                    }
                }
                _ => {}
            }
        }

        if decimal_pos < 0 {
            decimal_pos = digit_count;
        }

        if thousand_pos >= 0 {
            if thousand_pos == decimal_pos {
                scale_adjust -= thousand_count * 3;
            } else {
                thousand_seps = true;
            }
        }

        if number.digit(0) != 0 {
            number.scale += scale_adjust;

            let pos = if scientific {
                digit_count
            } else {
                number.scale + digit_count - decimal_pos
            };

            round_number(number, pos);
            if number.digit(0) == 0 {
                let zero = find_section(format, 2);

                if zero != section {
                    section = zero;
                    continue;
                }
            }
        } else {
            number.sign = false;
            number.scale = 0;
        }

        break;
    }

    first_digit = if first_digit < decimal_pos {
        decimal_pos - first_digit
    } else {
        0
    };
    last_digit = if last_digit > decimal_pos {
        decimal_pos - last_digit
    } else {
        0
    };

    let (mut dig_pos, mut adjust) = if scientific {
        (decimal_pos, 0)
    } else {
        (number.scale.max(decimal_pos), number.scale - decimal_pos)
    };
    let mut src = section;

    let mut separators: Vec<i32> = Vec::new();

    if thousand_seps && !info.number_group_separator.is_empty() {
        let groups = info.number_group_sizes;
        let mut group_index = 0;
        let mut group_total = groups.first().copied().unwrap_or(0);
        let mut group_size = group_total;
        let total_digits = dig_pos + adjust.min(0);
        let num_digits = first_digit.max(total_digits);

        while num_digits > group_total {
            if group_size == 0 {
                break;
            }

            separators.push(group_total);
            if group_index + 1 < groups.len() {
                group_index += 1;
                group_size = groups[group_index];
            }

            group_total += group_size;
        }
    }

    let separator_after = |separators: &mut Vec<i32>, out: &mut String, dig_pos: i32| {
        if thousand_seps && dig_pos > 1 && separators.last().is_some_and(|&at| dig_pos == at + 1) {
            out.push_str(info.number_group_separator);
            separators.pop();
        }
    };

    if number.sign && section == 0 {
        out.push_str(info.negative_sign);
    }

    let mut decimal_written = false;
    let mut cursor = 0;

    while src < len {
        let ch = format[src];

        src += 1;
        if ch == '\0' || ch == ';' {
            break;
        }

        if adjust > 0 && matches!(ch, '#' | '0' | '.') {
            while adjust > 0 {
                push_digit(out, number, &mut cursor);
                separator_after(&mut separators, out, dig_pos);
                dig_pos -= 1;
                adjust -= 1;
            }
        }

        match ch {
            '#' | '0' => {
                let digit = if adjust < 0 {
                    adjust += 1;

                    (dig_pos <= first_digit).then_some('0')
                } else {
                    next_digit(number, &mut cursor).or((dig_pos > last_digit).then_some('0'))
                };

                if let Some(digit) = digit {
                    out.push(digit);
                    separator_after(&mut separators, out, dig_pos);
                }

                dig_pos -= 1;
            }
            '.' => {
                if dig_pos != 0 || decimal_written {
                    continue;
                }

                if last_digit < 0 || (decimal_pos < digit_count && number.digit(cursor) != 0) {
                    out.push_str(info.number_decimal_separator);
                    decimal_written = true;
                }
            }
            '\u{2030}' => out.push_str(info.per_mille_symbol),
            '%' => out.push_str(info.percent_symbol),
            ',' => {}
            '\'' | '"' => {
                while src < len && format[src] != '\0' && format[src] != ch {
                    out.push(format[src]);
                    src += 1;
                }

                if src < len && format[src] != '\0' {
                    src += 1;
                }
            }
            '\\' => {
                if src < len && format[src] != '\0' {
                    out.push(format[src]);
                    src += 1;
                }
            }
            'E' | 'e' => {
                let at = |index: usize| format.get(index).copied();

                if scientific {
                    let mut positive_sign = false;
                    let mut zeros = 0;

                    if at(src) == Some('0') {
                        zeros += 1;
                    } else if at(src) == Some('+') && at(src + 1) == Some('0') {
                        positive_sign = true;
                    } else if !(at(src) == Some('-') && at(src + 1) == Some('0')) {
                        out.push(ch);
                        continue;
                    }

                    src += 1;
                    while at(src) == Some('0') {
                        src += 1;
                        zeros += 1;
                    }

                    let exponent = if number.digit(0) == 0 {
                        0
                    } else {
                        number.scale - decimal_pos
                    };

                    format_exponent(
                        out,
                        info,
                        exponent,
                        ch,
                        zeros.min(MAX_EXPONENT_DIGITS),
                        positive_sign,
                    );
                    scientific = false;
                } else {
                    out.push(ch);
                    if let Some(sign @ ('+' | '-')) = at(src) {
                        out.push(sign);
                        src += 1;
                    }

                    while at(src) == Some('0') {
                        out.push('0');
                        src += 1;
                    }
                }
            }
            _ => out.push(ch),
        }
    }
}

fn skip_quote_span(format: &[char], src: &mut usize, quote: char) {
    while *src < format.len() && format[*src] != '\0' {
        let ch = format[*src];

        *src += 1;
        if ch == quote {
            break;
        }
    }
}

fn find_section(format: &[char], mut section: i32) -> usize {
    if section == 0 {
        return 0;
    }

    let len = format.len();
    let mut src = 0;

    loop {
        if src >= len {
            return 0;
        }

        let ch = format[src];

        src += 1;

        match ch {
            '\'' | '"' => skip_quote_span(format, &mut src, ch),
            '\\' => {
                if src < len && format[src] != '\0' {
                    src += 1;
                }
            }
            ';' => {
                section -= 1;
                if section != 0 {
                    continue;
                }

                if src < len && format[src] != '\0' && format[src] != ';' {
                    return src;
                }

                return 0;
            }
            '\0' => return 0,
            _ => {}
        }
    }
}

fn round_number(number: &mut NumberBuffer, pos: i32) {
    let mut index = 0;

    while (index as i32) < pos && number.digit(index) != 0 {
        index += 1;
    }

    if index as i32 == pos && number.digit(index) >= b'5' {
        while index > 0 && number.digit(index - 1) == b'9' {
            index -= 1;
        }

        if index > 0 {
            let digit = number.digit(index - 1);

            number.set_digit(index - 1, digit + 1);
        } else {
            number.scale += 1;
            number.set_digit(0, b'1');
            index = 1;
        }
    } else {
        while index > 0 && number.digit(index - 1) == b'0' {
            index -= 1;
        }
    }

    if index == 0 {
        number.scale = 0;
        number.sign = false;
    }

    number.set_digit(index, 0);
}

fn format_currency(
    out: &mut String,
    number: &NumberBuffer,
    min_digits: i32,
    max_digits: i32,
    info: &NumberFormatInfo,
) {
    let pattern = if number.sign {
        NEGATIVE_CURRENCY_FORMATS[info.currency_negative_pattern]
    } else {
        POSITIVE_CURRENCY_FORMATS[info.currency_positive_pattern]
    };

    for ch in pattern.chars() {
        match ch {
            '#' => format_fixed_point(
                out,
                number,
                min_digits,
                max_digits,
                Some(info.currency_group_sizes),
                info.currency_decimal_separator,
                info.currency_group_separator,
            ),
            '-' => out.push_str(info.negative_sign),
            '$' => out.push_str(info.currency_symbol),
            _ => out.push(ch),
        }
    }
}

fn format_fixed_point(
    out: &mut String,
    number: &NumberBuffer,
    _min_digits: i32,
    mut max_digits: i32,
    groups: Option<&[i32]>,
    decimal_separator: &str,
    group_separator: &str,
) {
    let mut dig_pos = number.scale;
    let mut cursor = 0;

    if dig_pos > 0 {
        if let Some(groups) = groups {
            let mut group_size = 0;

            if let Some(&first) = groups.first() {
                let mut group_index = 0;
                let mut group_total = first;

                while dig_pos > group_total {
                    if groups[group_index] == 0 {
                        break;
                    }

                    if group_index + 1 < groups.len() {
                        group_index += 1;
                    }

                    group_total += groups[group_index];
                }

                group_size = if group_total == 0 { 0 } else { first };
            }

            let integer = usize::try_from(dig_pos).expect("positive");
            let start = integer.min(number.len());
            let mut digits_rev: Vec<char> = Vec::with_capacity(integer * 2);
            let mut group_index = 0;
            let mut digit_count = 0;

            for index in (0..integer).rev() {
                digits_rev.push(if index < start {
                    char::from(number.digit(index))
                } else {
                    '0'
                });
                if group_size > 0 {
                    digit_count += 1;
                    if digit_count == group_size && index != 0 {
                        digits_rev.extend(group_separator.chars().rev());
                        if group_index + 1 < groups.len() {
                            group_index += 1;
                            group_size = groups[group_index];
                        }

                        digit_count = 0;
                    }
                }
            }

            out.extend(digits_rev.iter().rev());
            cursor = start;
        } else {
            loop {
                push_digit(out, number, &mut cursor);
                dig_pos -= 1;
                if dig_pos <= 0 {
                    break;
                }
            }
        }
    } else {
        out.push('0');
    }

    if max_digits > 0 {
        out.push_str(decimal_separator);
        if dig_pos < 0 {
            let zeros = (-dig_pos).min(max_digits);

            out.extend(std::iter::repeat_n('0', zeros as usize));
            max_digits -= zeros;
        }

        while max_digits > 0 {
            push_digit(out, number, &mut cursor);
            max_digits -= 1;
        }
    }
}

fn format_number(
    out: &mut String,
    number: &NumberBuffer,
    min_digits: i32,
    max_digits: i32,
    info: &NumberFormatInfo,
) {
    let pattern = if number.sign {
        NEGATIVE_NUMBER_FORMATS[info.number_negative_pattern]
    } else {
        POSITIVE_NUMBER_FORMAT
    };

    for ch in pattern.chars() {
        match ch {
            '#' => format_fixed_point(
                out,
                number,
                min_digits,
                max_digits,
                Some(info.number_group_sizes),
                info.number_decimal_separator,
                info.number_group_separator,
            ),
            '-' => out.push_str(info.negative_sign),
            _ => out.push(ch),
        }
    }
}

fn format_scientific(
    out: &mut String,
    number: &NumberBuffer,
    _min_digits: i32,
    mut max_digits: i32,
    info: &NumberFormatInfo,
    exponent_char: char,
) {
    let mut cursor = 0;

    push_digit(out, number, &mut cursor);
    if max_digits != 1 {
        out.push_str(info.number_decimal_separator);
    }

    loop {
        max_digits -= 1;
        if max_digits <= 0 {
            break;
        }

        push_digit(out, number, &mut cursor);
    }

    let exponent = if number.digit(0) == 0 {
        0
    } else {
        number.scale - 1
    };

    format_exponent(out, info, exponent, exponent_char, 3, true);
}

fn format_exponent(
    out: &mut String,
    info: &NumberFormatInfo,
    value: i32,
    exponent_char: char,
    min_digits: i32,
    positive_sign: bool,
) {
    out.push(exponent_char);
    if value < 0 {
        out.push_str(info.negative_sign);
    } else if positive_sign {
        out.push_str(info.positive_sign);
    }

    let magnitude = value.unsigned_abs();

    if magnitude != 0 || min_digits > 0 {
        let width = usize::try_from(min_digits.max(0)).expect("not negative");
        let _ = write!(out, "{magnitude:0width$}");
    }
}

fn format_general(
    out: &mut String,
    number: &NumberBuffer,
    _min_digits: i32,
    max_digits: i32,
    info: &NumberFormatInfo,
    exponent_char: char,
    suppress_scientific: bool,
) {
    let mut dig_pos = number.scale;
    let mut scientific = false;

    if !suppress_scientific && (dig_pos > max_digits || dig_pos < -3) {
        dig_pos = 1;
        scientific = true;
    }

    let mut cursor = 0;

    if dig_pos > 0 {
        loop {
            push_digit(out, number, &mut cursor);
            dig_pos -= 1;
            if dig_pos <= 0 {
                break;
            }
        }
    } else {
        out.push('0');
    }

    if number.digit(cursor) != 0 || dig_pos < 0 {
        out.push_str(info.number_decimal_separator);
        while dig_pos < 0 {
            out.push('0');
            dig_pos += 1;
        }

        while let Some(digit) = next_digit(number, &mut cursor) {
            out.push(digit);
        }
    }

    if scientific {
        format_exponent(out, info, number.scale - 1, exponent_char, 2, true);
    }
}

fn format_percent(
    out: &mut String,
    number: &NumberBuffer,
    min_digits: i32,
    max_digits: i32,
    info: &NumberFormatInfo,
) {
    let pattern = if number.sign {
        NEGATIVE_PERCENT_FORMATS[info.percent_negative_pattern]
    } else {
        POSITIVE_PERCENT_FORMATS[info.percent_positive_pattern]
    };

    for ch in pattern.chars() {
        match ch {
            '#' => format_fixed_point(
                out,
                number,
                min_digits,
                max_digits,
                Some(info.percent_group_sizes),
                info.percent_decimal_separator,
                info.percent_group_separator,
            ),
            '-' => out.push_str(info.negative_sign),
            '%' => out.push_str(info.percent_symbol),
            _ => out.push(ch),
        }
    }
}
