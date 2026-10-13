use std::borrow::Cow;

use greybel_core::LiteralValue;
use greybel_core::syntax::Literal;
use greybel_symbols::literals::NULL;
use rustc_hash::FxHashMap;

use crate::emit::{LineBreakStyle, LiteralStyle, LiteralText};
use crate::names::NameTable;

const MIN_LENGTH: usize = 2;
const MIN_OCCURRENCES: usize = 2;
const MIN_TOTAL_LENGTH: usize = 10;

#[derive(Debug, Clone)]
struct Entry {
    occurrences: usize,
    total_length: usize,
    hoist: Option<HoistedLiteral>,
}

#[derive(Debug, Clone)]
struct HoistedLiteral {
    variable: Box<str>,
    literal: Literal,
    source: Box<str>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct LiteralTable {
    entries: Vec<Entry>,
    by_key: FxHashMap<Box<str>, usize>,
}

impl LiteralTable {
    pub(super) fn with_capacity(count: usize) -> Self {
        Self {
            entries: Vec::with_capacity(count),
            by_key: FxHashMap::with_capacity_and_hasher(count, Default::default()),
        }
    }

    pub(super) fn add(&mut self, literal: &Literal, source: &str, variables: &mut NameTable) {
        let key = key(literal, source);
        let length = if key.is_ascii() {
            key.len()
        } else {
            key.encode_utf16().count()
        };

        if length < MIN_LENGTH {
            return;
        }

        let Some(&index) = self.by_key.get(key.as_ref()) else {
            self.by_key.insert(key.as_ref().into(), self.entries.len());
            self.entries.push(Entry {
                occurrences: 1,
                total_length: length,
                hoist: None,
            });

            return;
        };
        let entry = &mut self.entries[index];

        entry.occurrences += 1;
        entry.total_length += length;
        if entry.hoist.is_none()
            && entry.occurrences > MIN_OCCURRENCES
            && entry.total_length > MIN_TOTAL_LENGTH
            && let Some(variable) = variables.create(&format!("\0{key}"))
        {
            entry.hoist = Some(HoistedLiteral {
                variable: variable.into(),
                literal: literal.clone(),
                source: source.into(),
            });
        }
    }

    pub(super) fn hoisted_variable(&self, literal: &Literal, source: &str) -> Option<&str> {
        let index = *self.by_key.get(key(literal, source).as_ref())?;
        let hoist = self.entries[index].hoist.as_ref()?;

        Some(&hoist.variable)
    }

    pub(super) fn hoisted_entries<'a>(
        &'a self,
        line_breaks: LineBreakStyle<'a>,
    ) -> impl Iterator<Item = (&'a str, String)> {
        let texts = self.entries.iter().filter_map(move |entry| {
            let hoist = entry.hoist.as_ref()?;
            let text = LiteralText::new(&hoist.literal, &hoist.source, LiteralStyle::Source)
                .with_line_breaks(line_breaks);

            Some((text.joins(), (&*hoist.variable, text.to_string())))
        });
        let (calls, plain): (Vec<_>, Vec<_>) = texts.partition(|(joins, _)| *joins);

        plain.into_iter().chain(calls).map(|(_, entry)| entry)
    }
}

fn key<'s>(literal: &Literal, source: &'s str) -> Cow<'s, str> {
    match (&literal.value, literal.negated) {
        (LiteralValue::Nil, _) => Cow::Borrowed(NULL),
        (LiteralValue::Boolean(value), negative) => Cow::Borrowed(match (negative, value) {
            (false, true) => "1",
            (false, false) => "0",
            (true, true) => "-1",
            (true, false) => "-0",
        }),
        (LiteralValue::Number(_), true) => Cow::Owned(format!("-{source}")),
        (LiteralValue::Number(_) | LiteralValue::String(_), _) => Cow::Borrowed(source),
    }
}

#[cfg(test)]
#[path = "tests/literals.rs"]
mod tests;
