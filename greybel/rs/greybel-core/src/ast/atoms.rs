use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Atom(u32);

impl Atom {
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Default)]
pub struct AtomTable {
    text: String,
    ends: Vec<u32>,
    by_hash: HashMap<u32, Atom, BuildHasherDefault<PassthroughHasher>>,
}

pub(crate) fn atom_hash(text: &str) -> u32 {
    let mut hasher = rustc_hash::FxHasher::default();

    hasher.write(text.as_bytes());

    (hasher.finish() >> 32) as u32
}

#[derive(Debug, Clone, Copy, Default)]
struct PassthroughHasher(u64);

impl Hasher for PassthroughHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = self.0.rotate_left(8) ^ u64::from(byte);
        }
    }

    fn write_u32(&mut self, value: u32) {
        self.0 = (u64::from(value) << 32) | u64::from(value);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

impl AtomTable {
    pub fn reserve(&mut self, additional: usize) {
        self.ends.reserve(additional);
        self.text.reserve(additional * 8);
        self.by_hash.reserve(additional);
    }

    pub fn insert(&mut self, text: &str) -> Atom {
        self.insert_with_hash(text, atom_hash(text))
    }

    pub(crate) fn insert_with_hash(&mut self, text: &str, hash: u32) -> Atom {
        if let Some(atom) = self.find(hash, text) {
            return atom;
        }

        let atom = Atom(u32::try_from(self.ends.len()).expect("fewer than 2^32 atoms"));

        self.text.push_str(text);
        self.ends
            .push(u32::try_from(self.text.len()).expect("atom texts are smaller than 4 GiB"));
        self.by_hash.entry(hash).or_insert(atom);

        atom
    }

    #[must_use]
    pub fn get(&self, text: &str) -> Option<Atom> {
        self.find(atom_hash(text), text)
    }

    fn find(&self, hash: u32, text: &str) -> Option<Atom> {
        let &atom = self.by_hash.get(&hash)?;
        let text = text.as_bytes();

        if Self::same(self.bytes_at(atom.0 as usize), text) {
            return Some(atom);
        }

        let index = (0..self.ends.len()).find(|&index| Self::same(self.bytes_at(index), text))?;

        Some(Atom(
            u32::try_from(index).expect("value fits the target integer type"),
        ))
    }

    fn same(atom_bytes: &[u8], text: &[u8]) -> bool {
        atom_bytes.len() == text.len() && atom_bytes.iter().zip(text).all(|(a, b)| a == b)
    }

    fn bytes_at(&self, index: usize) -> &[u8] {
        let start = index
            .checked_sub(1)
            .map_or(0, |previous| self.ends[previous]);

        &self.text.as_bytes()[start as usize..self.ends[index] as usize]
    }

    #[must_use]
    pub fn resolve(&self, atom: Atom) -> &str {
        self.text_at(atom.0 as usize)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.ends.len()
    }

    fn text_at(&self, index: usize) -> &str {
        let start = index
            .checked_sub(1)
            .map_or(0, |previous| self.ends[previous]);

        &self.text[start as usize..self.ends[index] as usize]
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }
}
