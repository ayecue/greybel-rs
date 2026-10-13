use std::iter::FusedIterator;
use std::slice;

use crate::dictionary::Entry;

#[derive(Debug, Clone)]
pub struct Iter<'a, K, V> {
    entries: slice::Iter<'a, Entry<K, V>>,
    vacancies: bool,
    remaining_count: usize,
}

impl<'a, K, V> Iter<'a, K, V> {
    pub(crate) fn new(entries: &'a [Entry<K, V>], vacancies: bool, len: usize) -> Self {
        Self {
            entries: entries.iter(),
            vacancies,
            remaining_count: len,
        }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let entry = if self.vacancies {
            self.entries.find(|entry| !entry.is_vacant())?
        } else {
            self.entries.next()?
        };

        self.remaining_count -= 1;

        Some(entry.pair())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining_count, Some(self.remaining_count))
    }

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        if self.vacancies {
            self.entries
                .filter(|entry| !entry.is_vacant())
                .fold(init, |acc, entry| f(acc, entry.pair()))
        } else {
            self.entries.fold(init, |acc, entry| f(acc, entry.pair()))
        }
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {}

impl<K, V> FusedIterator for Iter<'_, K, V> {}
