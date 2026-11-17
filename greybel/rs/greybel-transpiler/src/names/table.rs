use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::ShortNameIter;

#[derive(Debug, Clone)]
pub struct NameTable {
    candidates: ShortNameIter,
    aliases: FxHashMap<Arc<str>, Arc<str>>,
    owners: FxHashMap<Arc<str>, Arc<str>>,
    forbidden: FxHashSet<Box<str>>,
    buffer: String,
}

impl NameTable {
    #[must_use]
    pub fn new(characters: Vec<char>) -> Self {
        Self {
            candidates: ShortNameIter::new(characters),
            aliases: FxHashMap::default(),
            owners: FxHashMap::default(),
            forbidden: FxHashSet::default(),
            buffer: String::new(),
        }
    }

    #[must_use]
    pub fn with_capacity(characters: Vec<char>, count: usize) -> Self {
        let mut table = Self::new(characters);

        table.aliases.reserve(count);
        table.owners.reserve(count);

        table
    }

    #[must_use]
    pub fn forbid<S: Into<Box<str>>>(mut self, names: impl IntoIterator<Item = S>) -> Self {
        self.forbidden.extend(names.into_iter().map(Into::into));

        self
    }

    #[must_use]
    pub fn get(&self, original: &str) -> Option<&str> {
        self.aliases.get(original).map(AsRef::as_ref)
    }

    pub fn create(&mut self, original: &str) -> Option<&str> {
        self.insert(original);

        self.get(original)
    }

    #[must_use]
    pub fn next_length(&self) -> usize {
        let mut candidates = self.candidates.clone();
        let mut buffer = String::new();

        loop {
            candidates.write_next(&mut buffer);

            if !self.forbidden.contains(buffer.as_str())
                && !self.aliases.contains_key(buffer.as_str())
                && !self.owners.contains_key(buffer.as_str())
            {
                return buffer.chars().count();
            }
        }
    }

    pub fn insert(&mut self, original: &str) {
        if !self.aliases.contains_key(original) && !self.forbidden.contains(original) {
            self.assign(&Arc::from(original));
        }
    }

    fn assign(&mut self, original: &Arc<str>) {
        let Self {
            candidates,
            aliases,
            owners,
            forbidden,
            buffer,
        } = self;

        loop {
            candidates.write_next(buffer);

            let name = buffer.as_str();

            if !forbidden.contains(name)
                && !aliases.contains_key(name)
                && !owners.contains_key(name)
            {
                break;
            }
        }

        let name: Arc<str> = buffer.as_str().into();

        aliases.insert(original.clone(), name.clone());
        owners.insert(name, original.clone());
        if let Some(previous) = owners.remove(original) {
            self.assign(&previous);
        }
    }
}
