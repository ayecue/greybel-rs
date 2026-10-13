//! Word generation from first-name lists and [`MarkovNameGenerator`] models trained on them.

mod markov;

use std::sync::LazyLock;

use greyhack_world_assets::NameList;
pub(crate) use markov::MarkovNameGenerator;

use crate::Rng;
pub use crate::model::Gender;

/// Generated word category (`WordGenerator.word`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Word {
    Surname,
    Username,
    Corporation,
    Password,
    LibraryVariable,
    Exploit,
}

impl Word {
    const ALL: [Self; 6] = [
        Self::Surname,
        Self::Username,
        Self::Corporation,
        Self::Password,
        Self::LibraryVariable,
        Self::Exploit,
    ];
    const MIN_LENGTH: usize = 3;
    const ORDER: usize = 3;

    const fn list(self) -> NameList {
        match self {
            Self::Surname => NameList::Surnames,
            Self::Username => NameList::Usernames,
            Self::Corporation => NameList::Corporations,
            Self::Password => NameList::Passwords,
            Self::LibraryVariable => NameList::LibraryVariables,
            Self::Exploit => NameList::Exploits,
        }
    }

    fn generator(self) -> &'static MarkovNameGenerator {
        static GENERATORS: LazyLock<Vec<MarkovNameGenerator>> = LazyLock::new(|| {
            Word::ALL
                .iter()
                .map(|word| {
                    MarkovNameGenerator::new(
                        word.list().entries().iter().copied(),
                        Word::ORDER,
                        Word::MIN_LENGTH,
                    )
                })
                .collect()
        });

        &GENERATORS[self as usize]
    }

    /// Generates a word using caller-provided random indices.
    pub(crate) fn generate_with_index(self, index: impl FnMut(usize) -> usize) -> String {
        self.generator()
            .next_name_with(index)
            .trim_matches(['\'', '-'])
            .to_owned()
    }

    /// Generates a word without leading or trailing apostrophes or hyphens. Lowercases half of
    /// passwords.
    pub fn generate(self, rng: &mut Rng) -> String {
        let word = self
            .generator()
            .next_name(rng)
            .trim_matches(['\'', '-'])
            .to_owned();

        if self == Self::Password && rng.range(0, 1) == 1 {
            word.to_lowercase()
        } else {
            word
        }
    }
}

impl Gender {
    pub fn pick(rng: &mut Rng) -> Self {
        if rng.range(0, 1) == 0 {
            Self::Female
        } else {
            Self::Male
        }
    }

    /// Selects a first name from the gender list, excluding the trailing empty entry.
    pub fn first_name(self, rng: &mut Rng) -> String {
        self.first_name_with_index(|length| rng.index(length))
    }

    /// Selects a first name (`WordGenerator.GetNombre`) at the index `index` draws below the
    /// number of names.
    pub(crate) fn first_name_with_index(self, index: impl FnOnce(usize) -> usize) -> String {
        let list = match self {
            Self::Female => NameList::FemaleNames,
            Self::Male => NameList::MaleNames,
        };
        let names = list.entries();
        let names = names.strip_suffix(&[""]).unwrap_or(names);
        let name = names[index(names.len())];
        let mut chars = name.chars();

        chars.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(chars).collect()
        })
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
