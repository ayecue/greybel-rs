use std::hash::Hasher;

use rapidhash::fast::RapidHasher;

use super::{Heap, Number, Value};

const MAX_DEPTH: u32 = 4;

const NULL: u64 = 1;
const NUMBER: u64 = 2;
const STRING: u64 = 3;
const LIST: u64 = 4;
const MAP: u64 = 5;
const FUNCTION: u64 = 6;
const HOST_OBJECT: u64 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyHash(u64);

impl KeyHash {
    fn of(kind: u64, write: impl FnOnce(&mut RapidHasher)) -> Self {
        let mut hasher = RapidHasher::default();

        hasher.write_u64(kind);
        write(&mut hasher);

        Self(hasher.finish())
    }

    pub(crate) fn of_text(text: &str) -> Self {
        Self::of(STRING, |hasher| hasher.write(text.as_bytes()))
    }

    fn of_number(number: Number) -> Self {
        Self::of(NUMBER, |hasher| hasher.write_u64(number.key_bits()))
    }

    #[must_use]
    pub fn get(self) -> u64 {
        self.0
    }
}

impl Heap {
    #[must_use]
    pub fn key_hash(&self, value: &Value) -> KeyHash {
        self.recursive_key_hash(value, 0)
    }

    fn recursive_key_hash(&self, value: &Value, depth: u32) -> KeyHash {
        match value {
            Value::Null => KeyHash::of(NULL, |_| {}),
            Value::Number(number) => KeyHash::of_number(*number),
            Value::String(text) => KeyHash::of_text(text),
            Value::Function(id) => KeyHash::of(FUNCTION, |hasher| {
                hasher.write_u64(id.closure(self).body.identity());
            }),
            Value::List(id) => {
                let items = id.items(self);

                KeyHash::of(LIST, |hasher| {
                    hasher.write_usize(items.len());
                    if depth < MAX_DEPTH {
                        for item in items {
                            hasher.write_u64(self.recursive_key_hash(item, depth + 1).0);
                        }
                    }
                })
            }
            Value::Map(id) if id.is_host_object(self) => KeyHash::of(HOST_OBJECT, |hasher| {
                hasher.write_u32(id.identity());
            }),
            Value::Map(id) => {
                let map = id.entries(self);

                KeyHash::of(MAP, |hasher| {
                    hasher.write_usize(map.len());
                    if depth < MAX_DEPTH {
                        let entries = map.iter().fold(0u64, |sum, (key, value)| {
                            let mut entry = RapidHasher::default();

                            entry.write_u64(self.recursive_key_hash(key, depth + 1).0);
                            entry.write_u64(self.recursive_key_hash(value, depth + 1).0);

                            sum.wrapping_add(entry.finish())
                        });

                        hasher.write_u64(entries);
                    }
                })
            }
        }
    }
}
