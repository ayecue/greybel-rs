mod collator;
mod context;
mod contraction;
mod extender;
mod flags;
mod indexer;
mod kana;
mod table;

use std::cmp::Ordering;

use collator::len;
use table::TABLES;

#[must_use]
pub fn compare(text: &str, other: &str) -> Ordering {
    let text: Vec<u16> = text.encode_utf16().collect();
    let other: Vec<u16> = other.encode_utf16().collect();

    compare_units(&text, &other)
}

#[must_use]
pub fn compare_units(text: &[u16], other: &[u16]) -> Ordering {
    let tables = &*TABLES;

    tables
        .compare(text, 0, len(text), other, 0, len(other))
        .cmp(&0)
}

#[must_use]
pub fn ends_with(text: &str, suffix: &str) -> bool {
    let text: Vec<u16> = text.encode_utf16().collect();
    let suffix: Vec<u16> = suffix.encode_utf16().collect();

    ends_with_units(&text, &suffix)
}

#[must_use]
pub fn ends_with_units(text: &[u16], suffix: &[u16]) -> bool {
    if suffix.is_empty() || text == suffix {
        return true;
    }

    if text.is_empty() {
        return false;
    }

    TABLES.is_suffix(text, suffix)
}
