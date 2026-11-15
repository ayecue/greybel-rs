use greybel_core::Dialect;

use super::frame::Iteration;
use super::implementation::{GreyHack, Implementation, MiniScript};
use super::index::IndexResolver;
use super::{ValueType, Vm};
use crate::error::{IsaDepthLimit, RuntimeError, RuntimeErrorKind};
use crate::value::{EncodedText, Key, Map, Number, Value, ValueMap};

const MAX_ISA_DEPTH: usize = 256;

impl Vm {
    #[must_use]
    pub fn isa_of(&self, map: Map) -> Option<Map> {
        let isa = &self.keys.isa;
        let value = map
            .value_at(&self.heap, 0, isa)
            .or_else(|| map.get_key(&self.heap, isa));

        match value {
            Some(Value::Map(parent)) => Some(*parent),
            _ => None,
        }
    }

    pub fn isa_chain(&self, map: Map) -> impl Iterator<Item = Map> + '_ {
        std::iter::successors(Some(map), |&map| self.isa_of(map)).take(MAX_ISA_DEPTH)
    }

    pub fn map_lookup(
        &self,
        map: Map,
        key: &Value,
    ) -> Result<Option<(&Value, Map)>, IsaDepthLimit> {
        self.lookup_value(map, key, None)
    }

    #[inline(never)]
    pub fn map_lookup_by_key(
        &self,
        map: Map,
        key: &Key,
    ) -> Result<Option<(&Value, Map)>, IsaDepthLimit> {
        match self.program.dialect {
            Dialect::MiniScript => self.lookup_by_name::<MiniScript>(map, key),
            Dialect::GreyScript | Dialect::Greybel => self.lookup_by_name::<GreyHack>(map, key),
        }
    }

    #[inline(never)]
    fn lookup_by_name<I: Implementation>(
        &self,
        map: Map,
        key: &Key,
    ) -> Result<Option<(&Value, Map)>, IsaDepthLimit> {
        self.lookup_in_chain(map, Some(I::ISA_DEPTH), |entries| {
            Map::get_by_name(entries, key)
        })
    }

    #[inline(always)]
    fn lookup_value(
        &self,
        map: Map,
        key: &Value,
        name_depth: Option<usize>,
    ) -> Result<Option<(&Value, Map)>, IsaDepthLimit> {
        let hash = self.heap.key_hash(key).get();

        self.lookup_in_chain(map, name_depth, |entries| {
            entries
                .get(hash, |other| self.heap.equal(key, other))
                .map(|(_, value)| value)
        })
    }

    #[inline(always)]
    fn lookup_in_chain<'h>(
        &'h self,
        mut map: Map,
        name_depth: Option<usize>,
        find: impl Fn(&'h ValueMap) -> Option<&'h Value>,
    ) -> Result<Option<(&'h Value, Map)>, IsaDepthLimit> {
        let limit = name_depth.unwrap_or(MAX_ISA_DEPTH);
        let mut depth = 0;

        loop {
            let entries = map.entries(&self.heap);

            if let Some(value) = find(entries) {
                return Ok(Some((value, map)));
            }

            let Some(parent) = Map::parent_in(entries, &self.keys.isa) else {
                return if name_depth.is_some() && depth > limit {
                    Err(IsaDepthLimit)
                } else {
                    Ok(None)
                };
            };

            if depth > limit {
                return Err(IsaDepthLimit);
            }

            map = parent;
            depth += 1;
        }
    }

    pub(super) fn property<I: Implementation>(
        &self,
        base: &Value,
        key: &Value,
    ) -> Result<(Value, Option<Map>), RuntimeError> {
        if let Value::String(_) = key {
            return match base {
                Value::Null => Ok((Value::Null, None)),
                _ => self.resolve::<I>(base, key),
            };
        }

        match base {
            Value::Null | Value::Number(_) | Value::Function(_) => I::unindexable(base),
            Value::List(id) => {
                let index = Self::element_index::<I>(base, key)?;
                let items = id.items(&self.heap);

                IndexResolver::new(items.len())
                    .element(index)
                    .map(|at| (items[at].clone(), None))
                    .ok_or_else(|| I::out_of_range(self, base, key, index, items.len()))
            }
            Value::String(text) => {
                let index = Self::element_index::<I>(base, key)?;
                let units = text.units(self.heap.string_encoding());

                IndexResolver::new(units.len())
                    .element(index)
                    .map(|at| (Value::String(units.at(at)), None))
                    .ok_or_else(|| I::out_of_range(self, base, key, index, units.len()))
            }
            Value::Map(id) => match self.lookup_value(*id, key, None)? {
                Some((value, map)) => Ok((value.clone(), Some(map))),
                None if key.is_null() => Err(I::missing_null_key(self)),
                None => Err(self.missing_key(key)),
            },
        }
    }

    #[inline(always)]
    fn element_index<I: Implementation>(base: &Value, key: &Value) -> Result<i32, RuntimeError> {
        match key {
            Value::Number(_) => Ok(key.to_int()),
            _ => I::element_index(key, matches!(base, Value::List(_))),
        }
    }

    fn resolve<I: Implementation>(
        &self,
        base: &Value,
        key: &Value,
    ) -> Result<(Value, Option<Map>), RuntimeError> {
        let kind = match base {
            Value::Map(id) => match self.lookup_value(*id, key, Some(I::ISA_DEPTH)) {
                Ok(Some((value, map))) => return Ok((value.clone(), Some(map))),
                Ok(None) => ValueType::Map,
                Err(limit) => return I::isa_loop(limit),
            },
            Value::List(_) => ValueType::List,
            Value::String(_) => ValueType::String,
            Value::Number(_) => ValueType::Number,
            Value::Function(_) => ValueType::Function,
            Value::Null => return Ok((Value::Null, None)),
        };

        self.scopes
            .types
            .get(kind)
            .get(&self.heap, key)
            .map(|value| (value.clone(), None))
            .ok_or_else(|| self.missing_key(key))
    }

    pub(super) fn map_intrinsic(&self, key: &Value) -> Result<(Value, Option<Map>), RuntimeError> {
        if let Some(value) = self.scopes.types.map.get(&self.heap, key) {
            return Ok((value.clone(), None));
        }

        Err(self.missing_key(key))
    }

    pub(super) fn missing_key(&self, key: &Value) -> RuntimeError {
        RuntimeError::from(RuntimeErrorKind::MissingKey(
            self.heap.display(key).to_string(),
        ))
    }

    /// Reads `base.key` like [`Vm::property`] with a string key, except null has no members.
    #[inline(never)]
    pub(super) fn own_property<I: Implementation>(
        &self,
        base: &Value,
        key: &Key,
    ) -> Result<(Value, Option<Map>), RuntimeError> {
        let kind = match base {
            Value::Map(id) => match self.lookup_by_name::<I>(*id, key) {
                Ok(Some((value, map))) => return Ok((value.clone(), Some(map))),
                Ok(None) => ValueType::Map,
                Err(limit) => return I::isa_loop(limit),
            },
            Value::List(_) => ValueType::List,
            Value::String(_) => ValueType::String,
            Value::Number(_) => ValueType::Number,
            Value::Function(_) => ValueType::Function,
            Value::Null => {
                return Err(RuntimeError::from(RuntimeErrorKind::LookupInNull(
                    key.text().to_owned(),
                )));
            }
        };

        match self.scopes.types.get(kind).get_key(&self.heap, key) {
            Some(value) => Ok((value.clone(), None)),
            None => Err(self.missing_key(&key.value)),
        }
    }

    pub(super) fn set_property(
        &mut self,
        base: &Value,
        key: Value,
        value: Value,
    ) -> Result<(), RuntimeError> {
        match base {
            Value::Map(id) if id.is_sealed(&self.heap) => {
                Err(RuntimeError::from(RuntimeErrorKind::NotAssignable))
            }
            Value::Map(id) => {
                id.set(&mut self.heap, key, value);

                Ok(())
            }
            Value::List(id) => {
                if key.is_null() {
                    return Err(RuntimeError::from(RuntimeErrorKind::NullElementIndex));
                }

                let index = key.to_int();
                let mut items = id.items_mut(&mut self.heap);

                if let Some(at) = IndexResolver::new(items.len()).element(index) {
                    items[at] = value;

                    return Ok(());
                }

                drop(items);

                Err(RuntimeError::from(RuntimeErrorKind::ElementOutOfRange {
                    kind: base.type_name(),
                    index: self.heap.display(&key).to_string(),
                }))
            }
            Value::Null => Err(RuntimeError::from(RuntimeErrorKind::AssignmentToNull)),
            _ => Err(RuntimeError::from(RuntimeErrorKind::NotAssignable)),
        }
    }

    pub(super) fn is_a(&self, a: &Value, b: &Value) -> Result<bool, IsaDepthLimit> {
        let type_map = |kind| matches!(b, Value::Map(id) if *id == self.scopes.types.get(kind));

        if b.is_null() {
            return Ok(match a {
                Value::Null => true,
                Value::Number(_) => !self.is_named(ValueType::Number),
                Value::String(_) => !self.is_named(ValueType::String),
                Value::List(_) => !self.is_named(ValueType::List),
                Value::Map(_) => !self.is_named(ValueType::Map),
                Value::Function(_) => !self.is_named(ValueType::Function),
            });
        }

        Ok(match a {
            Value::Null => b.is_null(),
            Value::Number(_) => type_map(ValueType::Number),
            Value::String(_) => type_map(ValueType::String),
            Value::List(_) => type_map(ValueType::List),
            Value::Function(_) => type_map(ValueType::Function),
            Value::Map(id) => {
                let Value::Map(target) = b else {
                    return Ok(false);
                };

                if *target == self.scopes.types.map {
                    return Ok(true);
                }

                let mut parent = self.isa_of(*id);
                let mut depth = 0;

                while let Some(map) = parent {
                    if map == *target {
                        return Ok(true);
                    }

                    if depth > MAX_ISA_DEPTH {
                        return Err(IsaDepthLimit);
                    }

                    parent = self.isa_of(map);
                    depth += 1;
                }

                false
            }
        })
    }

    pub(super) fn iterator(&self, value: &Value) -> Iteration {
        match value {
            Value::List(id) => Iteration::List(*id),
            Value::Map(id) => Iteration::Map(*id),
            Value::String(text) => {
                Iteration::Chars(EncodedText::new(text, self.heap.string_encoding()))
            }
            Value::Number(_) | Value::Function(_) | Value::Null => Iteration::Empty,
        }
    }

    pub(super) fn iterator_value(&mut self, index: Number) -> Option<Value> {
        let index = index.to_index()?;
        let frame = self.calls.frames.last_mut().expect("the entry frame stays");
        let entry = match frame.iterations.last_mut()? {
            Iteration::List(id) => return id.items(&self.heap).get(index).cloned(),
            Iteration::Chars(units) => return units.get(index).map(Value::from),
            Iteration::Sequence(cursor) => return cursor.get(index),
            Iteration::Empty => return None,
            Iteration::Map(id) => {
                let (key, value) = id.entries(&self.heap).get_index(index)?;

                [
                    (&self.keys.entry_key, key.clone()),
                    (&self.keys.entry_value, value.clone()),
                ]
            }
        };

        Some(Value::Map(self.heap.add_map_from_keys(entry)))
    }
}
