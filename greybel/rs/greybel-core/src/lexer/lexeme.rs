use super::{Keyword, Punct};

pub(crate) trait Lexeme: Copy {
    fn text(self) -> &'static str;
}

impl Lexeme for Keyword {
    fn text(self) -> &'static str {
        self.as_str()
    }
}

impl Lexeme for Punct {
    fn text(self) -> &'static str {
        self.as_str()
    }
}

const BUCKETS: usize = 257;

#[derive(Debug, Clone)]
pub(crate) struct LexemeTable<T> {
    entries: Vec<(&'static str, T)>,
    starts: Box<[u16; BUCKETS]>,
    lengths: Box<[u32; BUCKETS]>,
}

impl<T: Lexeme> LexemeTable<T> {
    pub(crate) fn new(tables: &[&[T]]) -> Self {
        let mut entries: Vec<(&'static str, T)> = Vec::new();

        for &entry in tables.iter().flat_map(|table| table.iter()) {
            let text = entry.text();

            if !text.is_empty() && !entries.iter().any(|&(entry_text, _)| entry_text == text) {
                entries.push((text, entry));
            }
        }

        entries.sort_by(|(a, _), (b, _)| {
            let (a, b) = (a.as_bytes(), b.as_bytes());

            a[0].cmp(&b[0]).then(b.len().cmp(&a.len()))
        });

        let mut starts = Box::new([0u16; BUCKETS]);

        for (text, _) in &entries {
            starts[usize::from(text.as_bytes()[0]) + 1] += 1;
        }

        for byte in 1..BUCKETS {
            starts[byte] += starts[byte - 1];
        }

        let mut lengths = Box::new([0u32; BUCKETS]);

        for (text, _) in &entries {
            lengths[usize::from(text.as_bytes()[0])] |= Self::length_bit(text.len());
        }

        Self {
            entries,
            starts,
            lengths,
        }
    }

    fn length_bit(length: usize) -> u32 {
        1 << length.min(31)
    }

    fn bucket(&self, first: u8) -> &[(&'static str, T)] {
        let first = usize::from(first);

        &self.entries[usize::from(self.starts[first])..usize::from(self.starts[first + 1])]
    }

    pub(crate) fn longest_prefix(&self, input: &[u8]) -> Option<T> {
        let first = *input.first()?;

        self.bucket(first)
            .iter()
            .find(|(text, _)| {
                let text = text.as_bytes();

                input.len() >= text.len() && text.iter().zip(input).all(|(a, b)| a == b)
            })
            .map(|&(_, entry)| entry)
    }

    pub(crate) fn get(&self, word: &str) -> Option<T> {
        let word = word.as_bytes();
        let first = *word.first()?;

        if self.lengths[usize::from(first)] & Self::length_bit(word.len()) == 0 {
            return None;
        }

        self.bucket(first)
            .iter()
            .find(|(text, _)| {
                let text = text.as_bytes();

                text.len() == word.len() && text.iter().zip(word).all(|(a, b)| a == b)
            })
            .map(|&(_, entry)| entry)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CompoundStart {
    pub(crate) first: Keyword,
    pub(crate) alone: bool,
    pub(crate) endings: Vec<(&'static str, Keyword)>,
}

impl CompoundStart {
    pub(crate) fn keyword_for(&self, second: &str) -> Option<Keyword> {
        self.endings
            .iter()
            .find(|(text, _)| *text == second)
            .map(|&(_, keyword)| keyword)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CompoundKeywordTable {
    starts: Vec<CompoundStart>,
}

impl CompoundKeywordTable {
    pub(crate) fn new(tables: &[&[Keyword]]) -> Self {
        let keywords = || tables.iter().flat_map(|table| table.iter()).copied();
        let mut starts: Vec<CompoundStart> = Vec::new();

        for keyword in keywords() {
            let Some((first, second)) = keyword.as_str().split_once(' ') else {
                continue;
            };
            let index = match starts
                .iter()
                .position(|start| start.first.as_str() == first)
            {
                Some(index) => index,
                None => {
                    starts.push(CompoundStart {
                        first: Keyword::parse(first)
                            .expect("the first word of a compound keyword is a keyword"),
                        alone: keywords().any(|k| k.as_str() == first),
                        endings: Vec::new(),
                    });

                    starts.len() - 1
                }
            };

            if starts[index].keyword_for(second).is_none() {
                starts[index].endings.push((second, keyword));
            }
        }

        Self { starts }
    }

    pub(crate) fn first_words(&self) -> Vec<Keyword> {
        self.starts.iter().map(|start| start.first).collect()
    }

    pub(crate) fn start(&self, keyword: Keyword) -> Option<&CompoundStart> {
        self.starts.iter().find(|start| start.first == keyword)
    }
}

#[cfg(test)]
#[path = "tests/lexeme.rs"]
mod tests;
