//! Word generation from first-name lists and [`MarkovNameGenerator`] models trained on them
//! (`WordGenerator`).

mod gender;
mod markov;

use std::sync::LazyLock;

use greyhack_world_assets::NameList;
pub(crate) use markov::MarkovNameGenerator;

pub use self::gender::Gender;
use crate::Rng;

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
    pub fn generate_with_index(self, index: impl FnMut(usize) -> usize) -> String {
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
