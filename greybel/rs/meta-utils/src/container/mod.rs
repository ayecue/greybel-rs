mod build;
mod index;
mod search;

use std::collections::HashSet;
use std::sync::OnceLock;

use indexmap::{IndexMap, IndexSet};

use self::index::{Index, Slot};
use crate::definition::LocalizedDefinition;
use crate::signature::Signature;
use crate::type_meta::{BaseType, TypeLookup};
use crate::variation::VariationRegistry;

#[derive(Debug, Clone)]
pub struct Container {
    signatures: IndexMap<String, Signature>,
    search_exclusions: HashSet<String>,
    variations: VariationRegistry,
    index: OnceLock<Index>,
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Container {
    #[must_use]
    pub fn new() -> Self {
        let signatures = BaseType::ALL
            .into_iter()
            .map(|base| {
                let name = base.as_str().to_owned();

                (name.clone(), Signature::new(name))
            })
            .collect();

        Self {
            signatures,
            search_exclusions: HashSet::new(),
            variations: VariationRegistry::default(),
            index: OnceLock::new(),
        }
    }

    pub fn primitives(&self) -> impl Iterator<Item = &Signature> {
        self.signatures.values().take(BaseType::ALL.len())
    }

    pub fn types(&self) -> impl Iterator<Item = &Signature> {
        self.signatures.values().skip(BaseType::ALL.len())
    }

    pub fn signatures(&self) -> impl Iterator<Item = &Signature> {
        self.signatures.values()
    }

    #[must_use]
    pub fn signature(&self, type_name: &str) -> Option<&Signature> {
        self.signatures.get(type_name)
    }

    #[must_use]
    pub fn lookup_type(&self, name: &str) -> TypeLookup<'_> {
        if let Some(base) = BaseType::from_name(name) {
            TypeLookup::Base(base)
        } else if name == TypeLookup::NULL {
            TypeLookup::Null
        } else {
            self.signature(name)
                .map_or(TypeLookup::Unknown, TypeLookup::Class)
        }
    }

    pub fn visible_types(&self) -> impl Iterator<Item = &str> {
        self.visible_positions().map(|position| {
            self.signatures
                .get_index(position)
                .map(|(type_name, _)| type_name.as_str())
                .expect("visible positions are signature positions")
        })
    }

    pub fn members<'a>(
        &'a self,
        type_name: &str,
        language: &'a str,
    ) -> impl Iterator<Item = LocalizedDefinition<'a>> + use<'a> {
        let index = self.index();

        self.signatures
            .get_index_of(type_name)
            .into_iter()
            .flat_map(move |position| index.members(position).values())
            .map(move |slot| self.localize(*slot, language))
    }

    #[must_use]
    pub fn available_tags(&self) -> Vec<&str> {
        let tags: IndexSet<&str> = self
            .signatures()
            .flat_map(|signature| signature.definitions().values())
            .flat_map(|definition| definition.tags().iter().map(String::as_str))
            .collect();

        tags.into_iter().collect()
    }

    #[must_use]
    pub fn variations(&self) -> &VariationRegistry {
        &self.variations
    }

    fn visible_positions(&self) -> impl Iterator<Item = usize> {
        self.signatures
            .values()
            .enumerate()
            .filter(|(_, signature)| {
                !signature.is_hidden()
                    && signature.type_name() != BaseType::Any.as_str()
                    && signature.type_name() != BaseType::General.as_str()
            })
            .map(|(position, _)| position)
    }

    fn index(&self) -> &Index {
        self.index.get_or_init(|| Index::build(self))
    }

    fn localize(&self, slot: Slot, language: &str) -> LocalizedDefinition<'_> {
        let (_, signature) = self
            .signatures
            .get_index(slot.signature)
            .expect("index slots are signature positions");

        signature
            .definition_at(slot.definition, language)
            .expect("index slots are definition positions")
    }
}
