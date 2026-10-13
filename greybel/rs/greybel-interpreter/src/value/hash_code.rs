use rustc_hash::FxHashSet;

use super::{Heap, Value};

const NULL_KEY: i32 = -1;
const NULL_ELEMENT: i32 = 0;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Collection {
    List(super::List),
    Map(super::Map),
}

impl Heap {
    #[must_use]
    pub fn hash_code(&self, value: &Value) -> i32 {
        match value {
            Value::Map(id) if id.is_host_object(self) => self.element_hash(value, NULL_KEY),
            Value::List(_) | Value::Map(_) => self.recursive_hash(value),
            _ => self.element_hash(value, NULL_KEY),
        }
    }

    fn element_hash(&self, value: &Value, null_hash: i32) -> i32 {
        match value {
            Value::Null => null_hash,
            Value::Number(number) => mono_number::hash_code(number.get()),
            Value::String(text) => mono_string::hash_code(text),
            #[allow(clippy::cast_possible_truncation)]
            Value::Function(id) => id.closure(self).body.identity() as i32,
            #[allow(clippy::cast_possible_wrap)]
            Value::Map(id) if id.is_host_object(self) => id.identity() as i32,
            Value::List(_) | Value::Map(_) => self.recursive_hash(value),
        }
    }

    fn recursive_hash(&self, root: &Value) -> i32 {
        #[allow(clippy::manual_rotate)]
        let rotate = |hash: i32| (hash >> 1) | (hash << 31);
        let collection = |value: &Value| match value {
            Value::List(id) => Some(Collection::List(*id)),
            Value::Map(id) if !id.is_host_object(self) => Some(Collection::Map(*id)),
            _ => None,
        };
        let mut hash = 0;
        let mut visited_collections = FxHashSet::default();
        let mut stack = vec![(root.clone(), NULL_ELEMENT)];
        let push = |stack: &mut Vec<(Value, i32)>,
                    visited_collections: &FxHashSet<_>,
                    value: &Value,
                    null_hash| {
            if collection(value).is_none_or(|collection| !visited_collections.contains(&collection))
            {
                stack.push((value.clone(), null_hash));
            }
        };

        while let Some((value, null_hash)) = stack.pop() {
            if let Some(collection) = collection(&value) {
                visited_collections.insert(collection);
            }

            match &value {
                Value::List(id) => {
                    let items = id.items(self);

                    hash = rotate(hash) ^ i32::try_from(items.len()).unwrap_or(i32::MAX);
                    for item in items.iter().rev() {
                        push(&mut stack, &visited_collections, item, NULL_ELEMENT);
                    }
                }
                Value::Map(id) if !id.is_host_object(self) => {
                    let map = id.entries(self);

                    hash = rotate(hash) ^ i32::try_from(map.len()).unwrap_or(i32::MAX);
                    for (key, value) in map.iter() {
                        push(&mut stack, &visited_collections, key, NULL_KEY);
                        push(&mut stack, &visited_collections, value, NULL_ELEMENT);
                    }
                }
                other => hash = rotate(hash) ^ self.element_hash(other, null_hash),
            }
        }

        hash
    }
}
