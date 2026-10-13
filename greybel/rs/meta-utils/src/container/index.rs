use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use super::Container;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Slot {
    pub(super) signature: usize,
    pub(super) definition: usize,
}

#[derive(Debug, Clone)]
pub(super) struct Index {
    members: Vec<IndexMap<String, Slot>>,
    by_name: HashMap<String, Vec<Slot>>,
    by_id: HashMap<String, Vec<Slot>>,
}

impl Index {
    pub(super) fn build(container: &Container) -> Self {
        let members: Vec<_> = (0..container.signatures.len())
            .map(|position| inherited_members(container, position))
            .collect();

        let mut by_name: HashMap<String, Vec<Slot>> = HashMap::new();

        for position in container.visible_positions() {
            let (type_name, _) = container
                .signatures
                .get_index(position)
                .expect("visible positions are signature positions");

            if container.search_exclusions.contains(type_name) {
                continue;
            }

            for (name, slot) in &members[position] {
                let origins = by_name.entry(name.clone()).or_default();

                if !origins.contains(slot) {
                    origins.push(*slot);
                }
            }
        }

        let mut by_id: HashMap<String, Vec<Slot>> = HashMap::new();

        for (signature, (_, own)) in container.signatures.iter().enumerate() {
            for (definition, function) in own
                .definitions()
                .values()
                .enumerate()
                .filter_map(|(position, definition)| Some((position, definition.as_function()?)))
            {
                by_id
                    .entry(function.id().to_owned())
                    .or_default()
                    .push(Slot {
                        signature,
                        definition,
                    });
            }
        }

        Self {
            members,
            by_name,
            by_id,
        }
    }

    pub(super) fn members(&self, signature: usize) -> &IndexMap<String, Slot> {
        &self.members[signature]
    }

    pub(super) fn member(&self, signature: usize, name: &str) -> Option<Slot> {
        self.members[signature].get(name).copied()
    }

    pub(super) fn by_name(&self, name: &str) -> &[Slot] {
        self.by_name.get(name).map_or(&[], Vec::as_slice)
    }

    pub(super) fn by_id(&self, id: &str) -> &[Slot] {
        self.by_id.get(id).map_or(&[], Vec::as_slice)
    }
}

fn inherited_members(container: &Container, position: usize) -> IndexMap<String, Slot> {
    let mut members = IndexMap::new();
    let mut chain_positions = HashSet::new();
    let mut current = Some(position);

    while let Some(signature) = current.filter(|&signature| chain_positions.insert(signature)) {
        let (_, own) = container
            .signatures
            .get_index(signature)
            .expect("chain positions are signature positions");

        if !own.is_hidden() {
            for (definition, name) in own.definitions().keys().enumerate() {
                members.entry(name.clone()).or_insert(Slot {
                    signature,
                    definition,
                });
            }
        }

        current = own
            .extends()
            .and_then(|parent| container.signatures.get_index_of(parent));
    }

    members
}
