//! Mono-compatible UTF-16 case conversion using Mono's older Unicode tables.

mod tables;

use std::borrow::Cow;

use tables::{LOWER, UPPER};

#[must_use]
pub fn to_upper(text: &str) -> Cow<'_, str> {
    map(text, &UPPER, |c| c.to_ascii_uppercase())
}

#[must_use]
pub fn to_lower(text: &str) -> Cow<'_, str> {
    map(text, &LOWER, |c| c.to_ascii_lowercase())
}

fn map<'t>(text: &'t str, table: &[(u16, u16)], ascii: fn(char) -> char) -> Cow<'t, str> {
    let unit = |c: char| -> char {
        if c.is_ascii() {
            return ascii(c);
        }

        let Ok(code) = u16::try_from(u32::from(c)) else {
            return c;
        };

        table
            .binary_search_by_key(&code, |&(from, _)| from)
            .ok()
            .and_then(|at| char::from_u32(u32::from(table[at].1)))
            .unwrap_or(c)
    };

    match text.char_indices().find(|&(_, c)| unit(c) != c) {
        None => Cow::Borrowed(text),
        Some((first, _)) => {
            let mut out = String::with_capacity(text.len());

            out.push_str(&text[..first]);
            out.extend(text[first..].chars().map(unit));

            Cow::Owned(out)
        }
    }
}
