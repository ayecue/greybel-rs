use rustc_hash::FxHashMap;

const DENSE_IDS: u32 = 16384;

pub(crate) struct IdTable<T> {
    dense: Vec<Option<T>>,
    sparse: FxHashMap<u32, T>,
    len: usize,
}

impl<T> Default for IdTable<T> {
    fn default() -> Self {
        Self {
            dense: Vec::new(),
            sparse: FxHashMap::default(),
            len: 0,
        }
    }
}

impl<T> IdTable<T> {
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn insert(&mut self, id: u32, value: T) {
        let previous = if id < DENSE_IDS {
            let index = id as usize;

            if self.dense.len() <= index {
                self.dense.resize_with(index + 1, || None);
            }

            self.dense[index].replace(value)
        } else {
            self.sparse.insert(id, value)
        };

        if previous.is_none() {
            self.len += 1;
        }
    }

    pub(crate) fn get(&self, id: u32) -> Option<&T> {
        if id < DENSE_IDS {
            self.dense.get(id as usize)?.as_ref()
        } else {
            self.sparse.get(&id)
        }
    }

    pub(crate) fn remove_if(&mut self, id: u32, matches: impl FnOnce(&T) -> bool) -> Option<T> {
        let entry = if id < DENSE_IDS {
            let slot = self.dense.get_mut(id as usize)?;

            if !slot.as_ref().is_some_and(matches) {
                return None;
            }

            slot.take()
        } else {
            match self.sparse.entry(id) {
                std::collections::hash_map::Entry::Occupied(entry) if matches(entry.get()) => {
                    Some(entry.remove())
                }
                _ => None,
            }
        };

        if entry.is_some() {
            self.len -= 1;
        }

        entry
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        let dense = self.dense.iter().enumerate().filter_map(|(id, value)| {
            let id = u32::try_from(id).expect("dense ids are u32 ids");

            Some((id, value.as_ref()?))
        });

        dense.chain(self.sparse.iter().map(|(&id, value)| (id, value)))
    }

    pub(crate) fn drain(&mut self) -> Vec<T> {
        self.len = 0;

        let dense = std::mem::take(&mut self.dense);
        let sparse = std::mem::take(&mut self.sparse);

        dense
            .into_iter()
            .flatten()
            .chain(sparse.into_values())
            .collect()
    }
}

#[derive(Default)]
pub(crate) struct LocalIds {
    next: u32,
    vacant: Vec<u32>,
}

impl LocalIds {
    pub(crate) fn take(&mut self) -> u32 {
        self.vacant.pop().unwrap_or_else(|| {
            let id = self.next;

            self.next += 1;

            id
        })
    }

    pub(crate) fn release(&mut self, id: u32) {
        self.vacant.push(id);
    }
}
