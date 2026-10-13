use std::mem;

use hashbrown::HashTable;

use crate::iter::Iter;

#[derive(Debug, Clone)]
pub struct Dictionary<K, V> {
    entries: Vec<Entry<K, V>>,
    index: HashTable<usize>,
    #[expect(
        clippy::box_collection,
        reason = "one pointer; a dictionary is 64 bytes, a cache line; few have vacant slots"
    )]
    vacant_slots: Option<Box<Vec<usize>>>,
}

const SMALL: usize = 4;

const VACANT: u64 = u64::MAX;

#[derive(Debug, Clone)]
pub(crate) struct Entry<K, V> {
    hash: u64,
    key: K,
    value: V,
}

impl<K, V> Entry<K, V> {
    pub(crate) fn is_vacant(&self) -> bool {
        self.hash == VACANT
    }

    pub(crate) fn pair(&self) -> (&K, &V) {
        (&self.key, &self.value)
    }

    fn clamp_hash(hash: u64) -> u64 {
        hash.min(VACANT - 1)
    }
}

impl<K, V> Default for Dictionary<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Dictionary<K, V> {
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let index = if capacity > SMALL {
            HashTable::with_capacity(capacity)
        } else {
            HashTable::new()
        };

        Self {
            entries: Vec::with_capacity(capacity),
            index,
            vacant_slots: None,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len() - self.vacant_slots.as_deref().map_or(0, Vec::len)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    #[must_use]
    pub fn allocation_size(&self) -> usize {
        self.entries.capacity() * mem::size_of::<Entry<K, V>>()
            + self.index.allocation_size()
            + self.vacant_slots.as_deref().map_or(0, |vacant_slots| {
                vacant_slots.capacity() * mem::size_of::<usize>()
            })
    }

    pub fn reserve(&mut self, additional: usize) {
        self.entries.reserve(additional);
        if self.is_indexed() {
            let entries = &self.entries;

            self.index.reserve(additional, |&slot| entries[slot].hash);
        }
    }

    #[inline]
    pub fn find(&self, hash: u64, mut is_key: impl FnMut(&K) -> bool) -> Option<usize> {
        let hash = Entry::<K, V>::clamp_hash(hash);
        let entries = &self.entries;

        if self.is_indexed() {
            self.index
                .find(hash, |&slot| is_key(&entries[slot].key))
                .copied()
        } else {
            entries
                .iter()
                .position(|entry| entry.hash == hash && is_key(&entry.key))
        }
    }

    #[inline]
    pub fn get(&self, hash: u64, is_key: impl FnMut(&K) -> bool) -> Option<(&K, &V)> {
        let slot = self.find(hash, is_key)?;

        Some(self.entries[slot].pair())
    }

    #[inline]
    #[must_use]
    pub fn get_at(&self, slot: usize) -> Option<(&K, &V)> {
        let entry = self.entries.get(slot)?;

        (!entry.is_vacant()).then_some(entry.pair())
    }

    #[inline]
    pub fn get_at_mut(&mut self, slot: usize) -> Option<(&K, &mut V)> {
        let entry = self.entries.get_mut(slot)?;

        (!entry.is_vacant()).then_some((&entry.key, &mut entry.value))
    }

    #[must_use]
    pub fn get_index(&self, position: usize) -> Option<(&K, &V)> {
        if self.has_vacancies() {
            self.iter().nth(position)
        } else {
            self.entries.get(position).map(Entry::pair)
        }
    }

    #[must_use]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter::new(&self.entries, self.has_vacancies(), self.len())
    }

    pub fn keys(&self) -> impl ExactSizeIterator<Item = &K> {
        self.iter().map(|(key, _)| key)
    }

    pub fn values(&self) -> impl ExactSizeIterator<Item = &V> {
        self.iter().map(|(_, value)| value)
    }

    pub fn insert_unique(&mut self, hash: u64, key: K, value: V) -> usize {
        let hash = Entry::<K, V>::clamp_hash(hash);
        let vacant_slot = self.vacant_slots.as_deref_mut().and_then(Vec::pop);
        let slot = vacant_slot.unwrap_or(self.entries.len());

        if self.is_indexed() {
            let entries = &self.entries;

            self.index
                .insert_unique(hash, slot, |&slot| entries[slot].hash);
        }

        let entry = Entry { hash, key, value };

        if vacant_slot.is_some() {
            self.entries[slot] = entry;

            return slot;
        }

        if self.is_indexed() && self.entries.len() == self.entries.capacity() {
            let reserve_count = self.index.capacity().max(slot + 1) - slot;

            self.entries.reserve_exact(reserve_count);
        }

        self.entries.push(entry);
        if !self.is_indexed() && self.entries.len() > SMALL {
            self.build_index();
        }

        slot
    }

    #[inline]
    fn is_indexed(&self) -> bool {
        self.index.capacity() != 0
    }

    #[cold]
    fn build_index(&mut self) {
        let entries = &self.entries;

        self.index
            .reserve(entries.capacity(), |&slot| entries[slot].hash);
        for (slot, entry) in entries.iter().enumerate() {
            if !entry.is_vacant() {
                self.index
                    .insert_unique(entry.hash, slot, |&slot| entries[slot].hash);
            }
        }
    }

    pub fn remove_at(&mut self, slot: usize) -> Option<(K, V)>
    where
        K: Default,
        V: Default,
    {
        let entry = self.entries.get_mut(slot)?;

        if entry.is_vacant() {
            return None;
        }

        let hash = mem::replace(&mut entry.hash, VACANT);
        let removed_entry = (mem::take(&mut entry.key), mem::take(&mut entry.value));

        if self.is_indexed() {
            let Ok(index_entry) = self.index.find_entry(hash, |&other| other == slot) else {
                unreachable!("every occupied slot is indexed");
            };

            index_entry.remove();
        }

        self.vacant_slots.get_or_insert_default().push(slot);

        Some(removed_entry)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
        self.vacant_slots = None;
    }

    fn has_vacancies(&self) -> bool {
        self.vacant_slots
            .as_deref()
            .is_some_and(|vacant_slots| !vacant_slots.is_empty())
    }
}

impl<'a, K, V> IntoIterator for &'a Dictionary<K, V> {
    type IntoIter = Iter<'a, K, V>;
    type Item = (&'a K, &'a V);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
