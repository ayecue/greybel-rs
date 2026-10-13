use std::cmp::Ordering;

use greybel_interpreter::{EncodedText, Map, RuntimeError, Value, Vm, native};
use mono_random::Random;

use crate::error::IntrinsicError;

fn check_range(index: i64, min: i64, max: i64) -> Result<usize, IntrinsicError> {
    if (min..=max).contains(&index) {
        Ok(usize::try_from(index).unwrap_or_default())
    } else {
        Err(IntrinsicError::IndexOutOfRange { index, min, max })
    }
}

fn as_i64(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

fn in_range(index: &Value, len: usize) -> bool {
    let Value::Number(_) = index else {
        return false;
    };
    let index = i64::from(index.to_int());

    index >= -as_i64(len) && index < as_i64(len)
}

#[native(method)]
pub fn has_index(vm: &mut Vm, this: &Value, index: &Value) -> Value {
    let heap = vm.heap();

    match this {
        Value::List(id) => in_range(index, id.items(heap).len()).into(),
        Value::String(text) => in_range(index, heap.text_len(text)).into(),
        Value::Map(id) => id.contains(heap, index).into(),
        _ => Value::Null,
    }
}

fn numbers(len: usize) -> Vec<Value> {
    (0..len).map(|index| Value::from(index as f64)).collect()
}

#[native(method)]
pub fn indexes(vm: &mut Vm, this: &Value) -> Option<Vec<Value>> {
    let heap = vm.heap();

    match this {
        Value::Map(id) => Some(id.entries(heap).keys().cloned().collect()),
        Value::String(text) => Some(numbers(heap.text_len(text))),
        Value::List(id) => Some(numbers(id.items(heap).len())),
        _ => None,
    }
}

fn search_start(after: &Value, len: usize) -> Option<usize> {
    if after.is_null() {
        return Some(0);
    }

    let len = as_i64(len);
    let mut after = i64::from(after.to_int());

    if after < -1 {
        after += len;
    }

    if after < -1 || after >= len - 1 {
        return None;
    }

    usize::try_from(after + 1).ok()
}

#[native(method)]
pub fn index_of_value(vm: &mut Vm, this: &Value, value: &Value, after: &Value) -> Value {
    let heap = vm.heap();

    match this {
        Value::List(id) => {
            let items = id.items(heap);

            search_start(after, items.len())
                .and_then(|start| {
                    items[start..]
                        .iter()
                        .position(|item| heap.equal(item, value))
                        .map(|offset| Value::from((start + offset) as f64))
                })
                .unwrap_or_default()
        }
        Value::String(subject) => {
            if value.is_null() {
                return Value::Null;
            }

            let needle = heap.display(value).to_string();
            let units = heap.text_units(subject);

            search_start(after, units.len())
                .and_then(|start| units.find(&needle, start))
                .map_or(Value::Null, |position| Value::from(position as f64))
        }
        Value::Map(id) => {
            let mut seen_after = after.is_null();

            for (key, item) in id.entries(heap).iter() {
                if !seen_after {
                    seen_after = heap.equal(key, after);
                } else if heap.equal(item, value) {
                    return key.clone();
                }
            }

            Value::Null
        }
        _ => Value::Null,
    }
}

#[native(method)]
pub fn insert(
    vm: &mut Vm,
    this: &Value,
    index: &Value,
    value: &Value,
) -> Result<Value, IntrinsicError> {
    match index {
        Value::Null => return Err(IntrinsicError::InsertIndexMissing),
        Value::Number(_) => {}
        _ => return Err(IntrinsicError::InsertIndexNotANumber),
    }

    let mut index = i64::from(index.to_int());

    match this {
        Value::List(id) => {
            let len = as_i64(id.items(vm.heap()).len());

            if index < 0 {
                index += len + 1;
            }

            let at = check_range(index, 0, len)?;

            id.items_mut(vm.heap_mut()).insert(at, value.clone());

            Ok(this.clone())
        }
        Value::String(subject) => {
            let heap = vm.heap();
            let units = heap.text_units(subject);
            let len = as_i64(units.len());

            if index < 0 {
                index += len + 1;
            }

            let at = check_range(index, 0, len)?;
            let mut result = units.slice(0..at).to_string();

            result.push_str(&heap.display(value).to_string());
            result.push_str(&units.slice(at..units.len()));

            Ok(Value::from(result))
        }
        _ => Err(IntrinsicError::InsertType),
    }
}

#[native(method)]
pub fn join(vm: &mut Vm, this: &Value, #[default(" ")] delimiter: &Value) -> Value {
    let heap = vm.heap();
    let Value::List(id) = this else {
        return this.clone();
    };
    let text = |value: &Value| {
        if value.is_null() {
            String::new()
        } else {
            heap.display(value).to_string()
        }
    };
    let parts: Vec<String> = id.items(heap).iter().map(text).collect();
    let delimiter = text(delimiter);

    Value::from(parts.join(&delimiter))
}

#[native(method)]
pub fn len(vm: &mut Vm, this: &Value) -> Option<usize> {
    let heap = vm.heap();

    match this {
        Value::List(id) => Some(id.items(heap).len()),
        Value::String(text) => Some(heap.text_len(text)),
        Value::Map(id) => Some(id.entries(heap).len()),
        _ => None,
    }
}

fn take_first_key(vm: &mut Vm, map: Map) -> Value {
    let Some(key) = map.entries(vm.heap()).keys().next().cloned() else {
        return Value::Null;
    };

    map.remove(vm.heap_mut(), &key);

    key
}

#[native(method)]
pub fn pop(vm: &mut Vm, this: &Value) -> Value {
    match this {
        Value::List(id) => id.items_mut(vm.heap_mut()).pop().unwrap_or_default(),
        Value::Map(id) => take_first_key(vm, *id),
        _ => Value::Null,
    }
}

#[native(method)]
pub fn pull(vm: &mut Vm, this: &Value) -> Value {
    match this {
        Value::List(id) => {
            let mut items = id.items_mut(vm.heap_mut());

            if items.is_empty() {
                Value::Null
            } else {
                items.remove(0)
            }
        }
        Value::Map(id) => take_first_key(vm, *id),
        _ => Value::Null,
    }
}

#[native(method)]
pub fn push(vm: &mut Vm, this: &Value, value: &Value) -> Value {
    match this {
        Value::List(id) => id.items_mut(vm.heap_mut()).push(value.clone()),
        Value::Map(id) => id.set(vm.heap_mut(), value.clone(), Value::TRUE),
        _ => return Value::Null,
    }

    this.clone()
}

#[native(method)]
pub fn remove(vm: &mut Vm, this: &Value, key: &Value) -> Result<Value, IntrinsicError> {
    match this {
        Value::Map(id) => Ok(id.remove(vm.heap_mut(), key).is_some().into()),
        Value::List(id) => {
            if key.is_null() {
                return Err(IntrinsicError::RemoveNull);
            }

            let len = as_i64(id.items(vm.heap()).len());
            let mut index = i64::from(key.to_int());

            if index < 0 {
                index += len;
            }

            let at = check_range(index, 0, len - 1)?;

            id.items_mut(vm.heap_mut()).remove(at);

            Ok(Value::Null)
        }
        Value::String(subject) => {
            if key.is_null() {
                return Err(IntrinsicError::RemoveNull);
            }

            let part = vm.heap().display(key).to_string();

            Ok(match subject.find(part.as_str()) {
                Some(at) => {
                    let mut result = subject[..at].to_owned();

                    result.push_str(&subject[at + part.len()..]);

                    Value::from(result)
                }
                None => this.clone(),
            })
        }
        _ => Err(IntrinsicError::RemoveType),
    }
}

#[native(method)]
pub fn replace(
    vm: &mut Vm,
    this: &Value,
    oldval: &Value,
    newval: &Value,
    #[name = "maxCount"] max_count: &Value,
) -> Result<Value, IntrinsicError> {
    if this.is_null() {
        return Err(IntrinsicError::ReplaceNull);
    }

    let limit = if max_count.is_null() {
        usize::MAX
    } else {
        match usize::try_from(max_count.to_int()) {
            Ok(limit) if limit >= 1 => limit,
            _ => return Ok(this.clone()),
        }
    };

    match this {
        Value::Map(id) => {
            let heap = vm.heap();
            let keys: Vec<Value> = id
                .entries(heap)
                .iter()
                .filter(|(_, item)| heap.equal(item, oldval))
                .map(|(key, _)| key.clone())
                .take(limit)
                .collect();

            for key in keys {
                id.set(vm.heap_mut(), key, newval.clone());
            }

            Ok(this.clone())
        }
        Value::List(id) => {
            let heap = vm.heap();
            let indexes: Vec<usize> = id
                .items(heap)
                .iter()
                .enumerate()
                .filter(|(_, item)| heap.equal(item, oldval))
                .map(|(index, _)| index)
                .take(limit)
                .collect();
            let mut items = id.items_mut(vm.heap_mut());

            for index in indexes {
                items[index] = newval.clone();
            }

            Ok(this.clone())
        }
        Value::String(subject) => {
            let heap = vm.heap();
            let text = |value: &Value| {
                if value.is_null() {
                    String::new()
                } else {
                    heap.display(value).to_string()
                }
            };
            let old = text(oldval);

            if old.is_empty() {
                return Err(IntrinsicError::ReplaceEmpty);
            }

            let new = text(newval);

            Ok(Value::from(subject.replacen(old.as_str(), &new, limit)))
        }
        _ => Err(IntrinsicError::ReplaceType),
    }
}

fn bound(index: i64, len: i64) -> i64 {
    let index = if index < 0 { index + len } else { index };

    index.min(len)
}

#[native]
pub fn slice(vm: &mut Vm, seq: &Value, #[default(0)] from: i32, to: &Value) -> Value {
    let heap = vm.heap();
    let range = |len: usize| {
        let len = as_i64(len);
        let start = bound(i64::from(from), len).max(0);
        let end = if to.is_null() {
            len
        } else {
            bound(i64::from(to.to_int()), len)
        };
        let start = usize::try_from(start).unwrap_or_default();
        let end = usize::try_from(end).unwrap_or_default();

        start..end.max(start)
    };

    match seq {
        Value::List(id) => {
            let items = id.items(heap);
            let range = range(items.len());
            let slice = items[range].to_vec();

            Value::List(vm.heap_mut().add_list(slice))
        }
        Value::String(subject) => {
            let units = heap.text_units(subject);

            Value::String(units.slice(range(units.len())))
        }
        _ => Value::Null,
    }
}

#[native(method)]
pub fn sort(
    vm: &mut Vm,
    this: &Value,
    #[name = "byKey"] by_key: &Value,
    #[default(true)] ascending: bool,
) -> Result<Value, RuntimeError> {
    let Value::List(id) = this else {
        return Ok(this.clone());
    };
    let heap = vm.heap();
    let items = id.items(heap);

    if items.len() < 2 {
        return Ok(this.clone());
    }

    let mut key_pairs: Vec<(Value, Value)> = if by_key.is_null() {
        items
            .iter()
            .map(|item| (item.clone(), item.clone()))
            .collect()
    } else {
        let index = i64::from(by_key.to_int());

        items
            .iter()
            .map(|item| {
                let key = match item {
                    Value::Map(map) => vm
                        .map_lookup(*map, by_key)?
                        .map(|(value, _)| value.clone())
                        .unwrap_or_default(),
                    Value::List(list) => {
                        let values = list.items(heap);
                        let len = as_i64(values.len());

                        if index > -len && index < len {
                            let at = usize::try_from(index)
                                .map_err(|_| IntrinsicError::NegativeSortKey)?;

                            values[at].clone()
                        } else {
                            Value::Null
                        }
                    }
                    _ => Value::Null,
                };

                Ok((key, item.clone()))
            })
            .collect::<Result<_, RuntimeError>>()?
    };
    let order = |a: &(Value, Value), b: &(Value, Value)| -> Ordering {
        if ascending {
            heap.order(&a.0, &b.0)
        } else {
            heap.order(&b.0, &a.0)
        }
    };

    key_pairs.sort_by(order);

    let new_items = key_pairs.into_iter().map(|(_, item)| item).collect();

    *id.items_mut(vm.heap_mut()) = new_items;

    Ok(this.clone())
}

#[native(method)]
pub fn shuffle(vm: &mut Vm, this: &Value) {
    match this {
        Value::List(id) => {
            let len = id.items(vm.heap()).len();
            let random = vm.host_data_or_insert_with(Random::from_entropy);
            let swaps: Vec<usize> = (1..len).rev().map(|i| random.next_below(i + 1)).collect();
            let mut items = id.items_mut(vm.heap_mut());

            for (i, j) in (1..len).rev().zip(swaps) {
                items.swap(j, i);
            }
        }
        Value::Map(id) => {
            let entries: Vec<(Value, Value)> = id
                .entries(vm.heap())
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            let mut values: Vec<Value> = entries.iter().map(|(_, value)| value.clone()).collect();
            let random = vm.host_data_or_insert_with(Random::from_entropy);

            for i in (1..values.len()).rev() {
                values.swap(random.next_below(i + 1), i);
            }

            for ((key, _), value) in entries.into_iter().zip(values) {
                id.set(vm.heap_mut(), key, value);
            }
        }
        _ => {}
    }
}

#[native(method)]
pub fn sum(vm: &mut Vm, this: &Value) -> f64 {
    let heap = vm.heap();
    let add = |values: &mut dyn Iterator<Item = &Value>| {
        values
            .map(|value| match value {
                Value::Number(number) => number.get(),
                _ => 0.0,
            })
            .sum()
    };

    match this {
        Value::List(id) => add(&mut id.items(heap).iter()),
        Value::Map(id) => add(&mut id.entries(heap).values()),
        _ => 0.0,
    }
}

#[native(method)]
pub fn values(vm: &mut Vm, this: &Value) -> Value {
    let heap = vm.heap();
    let items: Vec<Value> = match this {
        Value::Map(id) => id.entries(heap).values().cloned().collect(),
        Value::String(subject) => {
            let units = EncodedText::new(subject, heap.string_encoding());

            (0..)
                .map_while(|at| units.get(at))
                .map(Value::from)
                .collect()
        }
        _ => return this.clone(),
    };

    Value::List(vm.heap_mut().add_list(items))
}
