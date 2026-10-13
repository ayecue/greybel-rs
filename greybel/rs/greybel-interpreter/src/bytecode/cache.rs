use std::cell::{Cell, OnceCell};

use crate::value::Map;

#[derive(Debug, Clone, Default)]
pub struct EntryCache(Cell<Option<(Map, u32)>>);

impl EntryCache {
    pub(crate) fn get(&self) -> Option<(Map, usize)> {
        self.0.get().map(|(map, index)| (map, index as usize))
    }

    pub(crate) fn set(&self, map: Map, index: usize) {
        if let Ok(index) = u32::try_from(index) {
            self.0.set(Some((map, index)));
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LookupCache {
    entry: EntryCache,
    chain: OnceCell<Box<Cell<Chain>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Chain {
    pub scope: Map,
    pub locals: Option<Map>,
    pub outer: Option<Map>,
    pub insertions: u32,
}

impl LookupCache {
    #[inline(always)]
    pub(crate) fn entry(&self) -> Option<(Map, usize)> {
        self.entry.get()
    }

    pub(crate) fn chain(&self) -> Option<Chain> {
        self.chain.get().map(|chain| chain.get())
    }

    pub(crate) fn set(&self, scope: Map, index: usize, chain: Option<Chain>) {
        self.entry.set(scope, index);
        if let Some(chain) = chain {
            self.chain
                .get_or_init(|| Box::new(Cell::new(chain)))
                .set(chain);
        }
    }
}
