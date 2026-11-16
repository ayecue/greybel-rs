use indexmap::IndexMap;
use meta_utils::BaseType;
use rustc_hash::FxHashSet;

use super::{
    CallContextId, CustomType, KeyRef, MapData, MapId, MapOrigin, SemanticModel, SlotId, SlotList,
};
use crate::types::{ApiFunction, TypeId, TypeTable};

impl SemanticModel {
    #[must_use]
    pub fn map(&self, id: MapId) -> &MapData {
        &self.maps[id.index()]
    }

    pub(crate) fn map_count(&self) -> usize {
        self.maps.len()
    }

    pub fn maps(&self) -> impl Iterator<Item = MapId> + use<'_> {
        (0..self.maps.len())
            .map(MapId::new)
            .filter(|&id| self.map(id).context.is_none())
    }

    pub(crate) fn map_of(&self, origin: MapOrigin) -> Option<MapId> {
        self.map_ids.get(&(origin, None)).copied()
    }

    pub(crate) fn insert_map(&mut self, origin: MapOrigin) -> MapId {
        self.insert_map_in(origin, None)
    }

    pub(crate) fn insert_map_in(
        &mut self,
        origin: MapOrigin,
        context: Option<CallContextId>,
    ) -> MapId {
        if let Some(&id) = self.map_ids.get(&(origin, context)) {
            return id;
        }

        let id = MapId::new(self.maps.len());
        let isa = self.add_slot();
        let dynamic_key = self.add_slot();
        let dynamic_value = self.add_slot();

        self.maps.push(MapData {
            origin,
            context,
            isa,
            entries: IndexMap::default(),
            dynamic_key,
            dynamic_value,
        });
        self.map_ids.insert((origin, context), id);

        id
    }

    #[must_use]
    pub fn prototype(&self, base: BaseType) -> Option<MapId> {
        self.map_ids
            .get(&(MapOrigin::Prototype(base), None))
            .copied()
    }

    pub(crate) fn singleton_type(&self, function: ApiFunction) -> Option<TypeId> {
        if function.origin != BaseType::General.as_str() {
            return None;
        }

        self.singleton_types.get(function.name).copied()
    }

    pub(crate) fn insert_singleton_type(&mut self, function: &'static str, ty: TypeId) {
        self.singleton_types.insert(function, ty);
    }

    pub(crate) fn insert_entry(&mut self, map: MapId, key: KeyRef<'_>) -> SlotId {
        if let Some(&slot) = self.maps[map.index()].entries.get(&key) {
            return slot;
        }

        let slot = self.add_slot();

        self.maps[map.index()].entries.insert(key.to_key(), slot);

        slot
    }

    #[must_use]
    pub fn isa_chain(&self, types: &TypeTable, map: MapId) -> Vec<MapId> {
        let mut lineage = Vec::new();
        let mut visited_maps = FxHashSet::default();
        let mut map_stack = vec![map];

        while let Some(map) = map_stack.pop() {
            if !visited_maps.insert(map) {
                continue;
            }

            lineage.push(map);

            let parent = self.slot_type(self.map(map).isa);
            let parents: Vec<MapId> = types.object_maps(parent).collect();

            map_stack.extend(parents.into_iter().rev());
        }

        lineage
    }

    #[must_use]
    pub(crate) fn visible_entry_slots(
        &self,
        types: &TypeTable,
        map: MapId,
        key: KeyRef<'_>,
    ) -> SlotList {
        let mut current = map;

        for _ in 0..=self.maps.len() {
            let data = self.map(current);

            if let Some(slot) = data.own_entry(key) {
                return SlotList::One(slot);
            }

            let parent = self.slot_type(data.isa);

            match types.object_map(parent) {
                Some(next) => current = next,
                None if types.object_maps(parent).next().is_none() => return SlotList::Empty,
                None => break,
            }
        }

        let mut slots = SlotList::Empty;
        let mut visited_maps = FxHashSet::default();
        let mut map_stack = vec![map];

        while let Some(map) = map_stack.pop() {
            if !visited_maps.insert(map) {
                continue;
            }

            let data = self.map(map);

            match data.own_entry(key) {
                Some(slot) => slots.add(slot),
                None => map_stack.extend(types.object_maps(self.slot_type(data.isa))),
            }
        }

        slots
    }

    pub fn custom_types(&self) -> impl Iterator<Item = &CustomType> {
        self.custom_types.values()
    }

    #[must_use]
    pub fn custom_type(&self, name: &str) -> Option<&CustomType> {
        self.custom_types.get(name)
    }

    #[must_use]
    pub fn custom_type_of(&self, map: MapId) -> Option<&CustomType> {
        self.custom_types().find(|ty| ty.map == map)
    }

    pub(crate) fn add_custom_type(&mut self, ty: CustomType) {
        self.custom_types.insert(ty.name.clone(), ty);
    }
}
