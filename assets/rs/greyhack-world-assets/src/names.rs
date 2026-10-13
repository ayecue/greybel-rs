use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NameList {
    MaleNames,
    FemaleNames,
    Surnames,
    Usernames,
    Corporations,
    Passwords,
    LibraryVariables,
    Exploits,
}

impl NameList {
    pub const ALL: [Self; 8] = [
        Self::MaleNames,
        Self::FemaleNames,
        Self::Surnames,
        Self::Usernames,
        Self::Corporations,
        Self::Passwords,
        Self::LibraryVariables,
        Self::Exploits,
    ];

    #[must_use]
    pub fn text(self) -> &'static str {
        if let Some(text) = REPLACEMENTS.get().and_then(|texts| texts.get(&self)) {
            return text;
        }

        crate::bundle::text(match self {
            Self::MaleNames => "names/male_names.txt",
            Self::FemaleNames => "names/female_names.txt",
            Self::Surnames => "names/surnames.txt",
            Self::Usernames => "names/usernames.txt",
            Self::Corporations => "names/corporations.txt",
            Self::Passwords => "names/passwords.txt",
            Self::LibraryVariables => "names/library_variables.txt",
            Self::Exploits => "names/exploits.txt",
        })
    }

    #[must_use]
    pub fn entries(self) -> &'static [&'static str] {
        &ENTRIES.get_or_init(|| {
            NameList::ALL
                .iter()
                .map(|list| list.text().split('\n').collect())
                .collect()
        })[self as usize]
    }

    /// Replaces the bundled lists with the game's own texts. The bundle stores the lists in a shuffled order, so the
    /// seeded draws from them (`WordGenerator.GetNombre`, `MarkovNameGenerator.NextName`) only match the game's when
    /// its texts are used. Returns `false` and changes nothing when the lists were already replaced or read.
    pub fn replace_texts(texts: impl IntoIterator<Item = (Self, String)>) -> bool {
        if ENTRIES.get().is_some() {
            return false;
        }

        REPLACEMENTS
            .set(
                texts
                    .into_iter()
                    .map(|(list, text)| (list, &*Box::leak(text.into_boxed_str())))
                    .collect(),
            )
            .is_ok()
    }

    /// The list the game loads as resource `name` (`TextFiles/Names/<name>`).
    #[must_use]
    pub fn of_resource(name: &str) -> Option<Self> {
        Some(match name {
            "male-names" => Self::MaleNames,
            "female-names" => Self::FemaleNames,
            "Surnamesmod" => Self::Surnames,
            "usernames" => Self::Usernames,
            "CorpNames" => Self::Corporations,
            "Passwords" => Self::Passwords,
            "lib_variables" => Self::LibraryVariables,
            "exploitNames" => Self::Exploits,
            _ => return None,
        })
    }
}

static REPLACEMENTS: OnceLock<HashMap<NameList, &'static str>> = OnceLock::new();
static ENTRIES: OnceLock<Vec<Vec<&'static str>>> = OnceLock::new();
