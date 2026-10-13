use std::rc::Rc;

use super::Heap;
use crate::value::{Key, KeyHash, Map, Value, ValueMap};

impl Heap {
    pub fn add_map(&mut self) -> Map {
        self.add_map_with_capacity(0)
    }

    pub fn add_map_with_capacity(&mut self, capacity: usize) -> Map {
        let map = match self.spare_maps.pop() {
            Some(mut map) => {
                map.reserve(capacity);

                map
            }
            None => ValueMap::with_capacity(capacity),
        };

        self.last_allocation_size += map.allocation_size();

        let map = self.maps.insert(map);

        if map as usize >= self.map_insertions.len() {
            self.map_insertions.resize(map as usize + 1, 0);
        }

        Map(map)
    }

    pub fn add_sealed_map(&mut self, entries: impl IntoIterator<Item = (Value, Value)>) -> Map {
        let map = self.add_map_from(entries);

        self.maps.seal(map.0);

        map
    }

    pub fn add_host_object<'k, T: 'static>(
        &mut self,
        entries: impl IntoIterator<Item = (&'k Key, Value)>,
        handle: T,
    ) -> Map {
        let map = self.add_map_from_keys(entries);

        self.maps.seal(map.0);
        self.hosts
            .get_or_insert_default()
            .insert(map.0, Rc::new(handle));

        map
    }

    pub fn add_map_from(&mut self, entries: impl IntoIterator<Item = (Value, Value)>) -> Map {
        let entries = entries.into_iter();
        let map = self.add_map_with_capacity(entries.size_hint().0);

        for (key, value) in entries {
            map.set(self, key, value);
        }

        map
    }

    pub fn add_map_from_keys<'k>(
        &mut self,
        entries: impl IntoIterator<Item = (&'k Key, Value)>,
    ) -> Map {
        let entries = entries.into_iter();
        let map = self.add_map_with_capacity(entries.size_hint().0);

        for (key, value) in entries {
            map.set_key_at(self, key, value);
        }

        map
    }
}

impl Map {
    #[must_use]
    pub fn entries(self, heap: &Heap) -> &ValueMap {
        heap.maps.get(self.0)
    }

    fn entries_mut(self, heap: &mut Heap) -> &mut ValueMap {
        heap.maps.get_mut(self.0)
    }

    #[inline(always)]
    pub(crate) fn insertions(self, heap: &Heap) -> u32 {
        heap.map_insertions[self.0 as usize]
    }

    #[must_use]
    pub fn is_sealed(self, heap: &Heap) -> bool {
        heap.maps.is_sealed(self.0)
    }

    /// Checks whether this map is a host object ([`Heap::add_host_object`]). Like Grey Hack game objects
    /// (`GreyObject`), host objects hash and compare by identity, so only the same object compares equal.
    #[must_use]
    pub fn is_host_object(self, heap: &Heap) -> bool {
        heap.hosts
            .as_deref()
            .is_some_and(|hosts| hosts.contains_key(&self.0))
    }

    /// Checks whether the map remains on the heap; a freed map's index may belong to a new map.
    #[must_use]
    pub fn is_live(self, heap: &Heap) -> bool {
        heap.maps.contains(self.0)
    }

    /// Heap index identifying this map for its lifetime.
    #[must_use]
    pub(crate) fn identity(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn handle<T: 'static>(self, heap: &Heap) -> Option<&T> {
        heap.hosts.as_deref()?.get(&self.0)?.downcast_ref()
    }

    #[inline(always)]
    fn matches_name(key: &Key, value: &Value) -> bool {
        match (value, &key.value) {
            (Value::String(other), Value::String(name)) => other == name,
            (Value::String(other), _) => other.is_empty(),
            _ => false,
        }
    }

    fn index(self, heap: &Heap, hash: KeyHash, key: &Value) -> Option<usize> {
        self.entries(heap)
            .find(hash.get(), |other| heap.equal(key, other))
    }

    pub(crate) fn key_index(self, heap: &Heap, key: &Key) -> Option<usize> {
        self.entries(heap)
            .find(key.hash.get(), |other| Self::matches_name(key, other))
    }

    #[must_use]
    pub fn get<'h>(self, heap: &'h Heap, key: &Value) -> Option<&'h Value> {
        self.entries(heap)
            .get(heap.key_hash(key).get(), |other| heap.equal(key, other))
            .map(|(_, value)| value)
    }

    #[must_use]
    pub fn get_key<'h>(self, heap: &'h Heap, key: &Key) -> Option<&'h Value> {
        self.entries(heap)
            .get(key.hash.get(), |other| Self::matches_name(key, other))
            .map(|(_, value)| value)
    }

    #[must_use]
    pub fn get_str<'h>(self, heap: &'h Heap, key: &str) -> Option<&'h Value> {
        self.entries(heap)
            .get(
                KeyHash::of_text(key).get(),
                |other| matches!(other, Value::String(text) if **text == *key),
            )
            .map(|(_, value)| value)
    }

    #[must_use]
    pub fn contains(self, heap: &Heap, key: &Value) -> bool {
        self.index(heap, heap.key_hash(key), key).is_some()
    }

    pub fn set(self, heap: &mut Heap, key: Value, value: Value) {
        let hash = heap.key_hash(&key);

        match self.index(heap, hash, &key) {
            Some(index) => self.set_at(heap, index, value),
            None => {
                self.insert(heap, hash, key, value);
            }
        }
    }

    pub fn set_key(self, heap: &mut Heap, key: &Key, value: Value) {
        self.set_key_at(heap, key, value);
    }

    pub fn remove(self, heap: &mut Heap, key: &Value) -> Option<Value> {
        let index = self.index(heap, heap.key_hash(key), key)?;

        self.entries_mut(heap)
            .remove_at(index)
            .map(|(_, value)| value)
    }

    pub fn clear(self, heap: &mut Heap) {
        self.entries_mut(heap).clear();
    }

    pub fn merge(self, heap: &mut Heap, other: Self) -> Self {
        let entries: Vec<_> = self
            .entries(heap)
            .iter()
            .chain(other.entries(heap).iter())
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();

        heap.add_map_from(entries)
    }

    #[inline(always)]
    pub(crate) fn get_by_name<'h>(entries: &'h ValueMap, key: &Key) -> Option<&'h Value> {
        entries
            .get(key.hash.get(), |other| Self::matches_name(key, other))
            .map(|(_, value)| value)
    }

    #[inline(always)]
    pub(crate) fn parent_in(entries: &ValueMap, isa: &Key) -> Option<Self> {
        let value = entries
            .get_at(0)
            .filter(|(entry, _)| Self::matches_name(isa, entry))
            .or_else(|| entries.get(isa.hash.get(), |other| Self::matches_name(isa, other)));

        match value {
            Some((_, Value::Map(parent))) => Some(*parent),
            _ => None,
        }
    }

    #[inline(always)]
    pub(crate) fn value_at<'h>(self, heap: &'h Heap, index: usize, key: &Key) -> Option<&'h Value> {
        let (entry, value) = self.entries(heap).get_at(index)?;

        Self::matches_name(key, entry).then_some(value)
    }

    #[inline(always)]
    pub(crate) fn set_at(self, heap: &mut Heap, index: usize, value: Value) {
        if let Some((_, old)) = self.entries_mut(heap).get_at_mut(index) {
            *old = value;
        }
    }

    pub(crate) fn set_key_at(self, heap: &mut Heap, key: &Key, value: Value) -> usize {
        match self.key_index(heap, key) {
            Some(index) => {
                self.set_at(heap, index, value);

                index
            }
            None => self.insert(heap, key.hash, key.value.clone(), value),
        }
    }

    fn insert(self, heap: &mut Heap, hash: KeyHash, key: Value, value: Value) -> usize {
        let Heap {
            maps,
            last_allocation_size: allocation_size,
            map_insertions,
            ..
        } = heap;
        let insertions = &mut map_insertions[self.0 as usize];

        *insertions = insertions.saturating_add(1);

        let entries = maps.get_mut(self.0);
        let before = entries.allocation_size();
        let index = entries.insert_unique(hash.get(), key, value);

        *allocation_size += entries.allocation_size().saturating_sub(before);

        index
    }
}
