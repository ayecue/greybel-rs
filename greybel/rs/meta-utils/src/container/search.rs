use indexmap::IndexMap;

use super::Container;
use super::index::Slot;
use crate::definition::LocalizedDefinition;
use crate::type_meta::BaseType;

impl Container {
    #[must_use]
    pub fn search_definitions<S: AsRef<str>>(
        &self,
        types: &[S],
        property: &str,
        language: &str,
    ) -> IndexMap<&str, LocalizedDefinition<'_>> {
        let mut matches = IndexMap::new();

        for slot in self.search(types, property) {
            let localized_definition = self.localize(slot, language);

            matches
                .entry(localized_definition.origin())
                .or_insert(localized_definition);
        }

        matches
    }

    #[must_use]
    pub fn definition<S: AsRef<str>>(
        &self,
        types: &[S],
        property: &str,
        language: &str,
    ) -> Option<LocalizedDefinition<'_>> {
        let any = BaseType::Any.as_str();
        let any_definition = self
            .signature(any)
            .and_then(|signature| signature.definition(property, language));

        if any_definition.is_some() && includes_any(types) {
            return any_definition;
        }

        let any_position = self.signatures.get_index_of(any);
        let mut slots = self.search(types, property);
        let first = slots.next()?;
        let mut found_in_several_types = false;
        let mut on_any = (Some(first.signature) == any_position).then_some(first);

        for slot in slots {
            found_in_several_types |= slot.signature != first.signature;
            if Some(slot.signature) == any_position {
                on_any = Some(slot);
            }
        }

        if found_in_several_types {
            if let Some(slot) = on_any {
                return Some(self.localize(slot, language));
            }

            if any_definition.is_some() {
                return any_definition;
            }
        }

        Some(self.localize(first, language))
    }

    pub fn definitions_by_id<'a>(
        &'a self,
        id: &str,
        language: &'a str,
    ) -> impl Iterator<Item = LocalizedDefinition<'a>> + use<'a> {
        self.index()
            .by_id(id)
            .iter()
            .map(move |slot| self.localize(*slot, language))
    }

    fn search<'a, S: AsRef<str>>(
        &'a self,
        types: &'a [S],
        property: &'a str,
    ) -> impl Iterator<Item = Slot> + 'a {
        let index = self.index();
        let everything = includes_any(types);
        let visible = if everything {
            index.by_name(property)
        } else {
            &[]
        };
        let named_slots = types
            .iter()
            .filter(move |_| !everything)
            .map(AsRef::as_ref)
            .filter(|type_name| !self.search_exclusions.contains(*type_name))
            .filter_map(|type_name| self.signatures.get_index_of(type_name))
            .filter_map(move |position| index.member(position, property));

        visible.iter().copied().chain(named_slots)
    }
}

fn includes_any<S: AsRef<str>>(types: &[S]) -> bool {
    types
        .iter()
        .any(|type_name| type_name.as_ref() == BaseType::Any.as_str())
}
