use std::borrow::Cow;
use std::ops::Range;

const REPLACEMENT: char = '\u{FFFD}';
const PAIR_BYTES: usize = 4;

#[must_use]
pub fn len(text: &str) -> usize {
    if text.is_ascii() {
        return text.len();
    }

    str_indices::utf16::count(text)
}

#[must_use]
pub fn unit_index(text: &str, byte: usize) -> usize {
    if text.is_ascii() {
        return byte.min(text.len());
    }

    str_indices::utf16::from_byte_idx(text, byte)
}

#[must_use]
pub fn byte_index(text: &str, unit: usize) -> usize {
    if text.is_ascii() {
        return unit.min(text.len());
    }

    str_indices::utf16::to_byte_idx(text, unit)
}

#[must_use]
pub fn substring(text: &str, units: Range<usize>) -> Cow<'_, str> {
    if units.start >= units.end {
        return Cow::Borrowed("");
    }

    if text.is_ascii() {
        let end = units.end.min(text.len());

        return Cow::Borrowed(&text[units.start.min(end)..end]);
    }

    let mut start = byte_index(text, units.start);
    let starts_inside = unit_index(text, start) != units.start;

    if starts_inside {
        start += PAIR_BYTES;
    }

    let end = byte_index(text, units.end);
    let ends_inside = unit_index(text, end) != units.end;

    if !starts_inside && !ends_inside {
        return Cow::Borrowed(&text[start..end]);
    }

    let mut out = String::with_capacity(end - start.min(end) + 2 * REPLACEMENT.len_utf8());

    if starts_inside {
        out.push(REPLACEMENT);
    }

    out.push_str(&text[start.min(end)..end]);
    if ends_inside {
        out.push(REPLACEMENT);
    }

    Cow::Owned(out)
}

#[must_use]
pub fn index_of(text: &str, value: &str) -> Option<usize> {
    text.find(value).map(|byte| unit_index(text, byte))
}

#[must_use]
pub fn last_index_of(text: &str, value: &str) -> Option<usize> {
    if value.is_empty() {
        return Some(len(text).saturating_sub(1));
    }

    text.rfind(value).map(|byte| unit_index(text, byte))
}
