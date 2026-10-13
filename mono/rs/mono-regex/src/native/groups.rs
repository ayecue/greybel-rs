//! Implements .NET group numbering, including forward references.

use std::collections::{BTreeMap, BTreeSet};

use rustc_hash::FxHashMap;

use super::ast::Node;
use super::class;
use super::cursor::{Cursor, is_word_char};
use super::error::Reason;
use super::options::Options;

#[derive(Debug, Default)]
pub struct SlotTable {
    numbers: BTreeSet<u32>,
    names: FxHashMap<String, u32>,
}

impl SlotTable {
    pub fn count(pattern: &str, mut options: Options) -> Result<Self, Reason> {
        let mut cursor = Cursor::new(pattern);
        let mut slots = Self::default();
        let mut names = Vec::new();
        let mut group_stack = Vec::new();
        let mut autocap = 1;
        let mut condition_next = false;

        loop {
            let start = cursor.position();
            let Some(c) = cursor.bump() else { break };

            match c {
                '\\' if cursor.peek().is_some() => skip_escape(&mut cursor)?,
                '#' if options.ignore_whitespace => {
                    cursor.reset(start);
                    cursor.skip_blank(true)?;
                }
                '[' => class::skip(&mut cursor)?,
                ')' => {
                    if let Some(outer) = group_stack.pop() {
                        options = outer;
                    }
                }
                '(' if cursor.remainder().starts_with("?#") => {
                    cursor.reset(start);
                    cursor.skip_blank(options.ignore_whitespace)?;
                }
                '(' => {
                    group_stack.push(options);

                    let condition = std::mem::take(&mut condition_next);

                    if !cursor.consume('?') {
                        if !(options.explicit_capture || condition) {
                            slots.numbers.insert(autocap);
                            autocap += 1;
                        }
                    } else if matches!(cursor.peek(), Some('<' | '\''))
                        && cursor.peek_nth(1).is_some()
                    {
                        cursor.bump();

                        match cursor.peek() {
                            Some('1'..='9') => {
                                slots.numbers.insert(cursor.decimal()?);
                            }
                            Some(c) if c != '0' && is_word_char(c) => names.push(cursor.name()),
                            _ => {}
                        }
                    } else {
                        options.scan(&mut cursor);

                        match cursor.peek() {
                            Some(')') => {
                                cursor.bump();
                                group_stack.pop();
                            }
                            Some('(') => condition_next = true,
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        let mut next = autocap;

        for name in names {
            if slots.names.contains_key(name) {
                continue;
            }

            while slots.numbers.contains(&next) {
                next += 1;
            }

            slots.numbers.insert(next);
            slots.names.insert(name.to_owned(), next);
            next += 1;
        }

        Ok(slots)
    }

    pub fn contains(&self, number: u32) -> bool {
        number == 0 || self.numbers.contains(&number)
    }

    pub fn number(&self, name: &str) -> Option<u32> {
        self.names.get(name).copied()
    }
}

fn skip_escape(cursor: &mut Cursor<'_>) -> Result<(), Reason> {
    if cursor.peek().is_some_and(|c| "bBAGZzwWsSdDpP".contains(c)) {
        cursor.bump();

        return Ok(());
    }

    let start = cursor.position();

    if cursor.backreference()?.is_none() {
        cursor.reset(start);
        cursor.char_escape()?;
    }

    Ok(())
}

#[derive(Debug, Default)]
pub struct GroupTable {
    indices: BTreeMap<u32, usize>,
    names: FxHashMap<String, u32>,
}

impl GroupTable {
    pub fn new(node: &Node, slots: SlotTable) -> Result<Self, Reason> {
        let mut indices = BTreeMap::new();

        for (index, number) in (1..).zip(node.captures()) {
            if indices.insert(number, index).is_some() {
                let name = slots
                    .names
                    .iter()
                    .find(|&(_, &slot_number)| slot_number == number)
                    .map_or_else(|| number.to_string(), |(name, _)| name.clone());

                return Err(Reason::DuplicateGroup(name));
            }
        }

        Ok(Self {
            indices,
            names: slots.names,
        })
    }

    pub fn index(&self, number: u32) -> Option<usize> {
        match number {
            0 => Some(0),
            number => self.indices.get(&number).copied(),
        }
    }

    pub fn number(&self, name: &str) -> Option<u32> {
        self.names.get(name).copied()
    }

    pub fn last(&self) -> u32 {
        self.indices.keys().next_back().copied().unwrap_or(0)
    }

    pub fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.indices.values().copied()
    }
}
