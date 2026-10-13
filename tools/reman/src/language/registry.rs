//! Languages reman knows, found by name in `reman.toml` files and on the command line. A language is compared,
//! ordered, hashed and written by its name.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Language;
use super::dotnet::Dotnet;
use super::duplex::Duplex;
use super::rust::Rust;
use super::typescript::Typescript;

/// All languages supported by reman.
pub struct LanguageRegistry;

impl LanguageRegistry {
    pub const ALL: [&'static dyn Language; 4] = [&Rust, &Typescript, &Dotnet, &Duplex];

    /// Language `text` names, by its name (`rust`) or id prefix (`rs`).
    pub fn by_name(text: &str) -> Result<&'static dyn Language> {
        Self::ALL
            .into_iter()
            .find(|language| language.name() == text || language.prefix() == text)
            .with_context(|| {
                let names = Self::ALL.map(|language| language.name());

                format!("unknown language `{text}`: {}", names.join(", "))
            })
    }
}

/// Language in a `reman.toml` file is its name.
impl<'de> Deserialize<'de> for &'static dyn Language {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;

        LanguageRegistry::by_name(&name).map_err(serde::de::Error::custom)
    }
}

impl Serialize for dyn Language {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl PartialEq for dyn Language {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
    }
}

impl Eq for dyn Language {}

impl PartialOrd for dyn Language {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for dyn Language {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name().cmp(other.name())
    }
}

impl Hash for dyn Language {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name().hash(state);
    }
}

impl fmt::Debug for dyn Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl fmt::Display for dyn Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
