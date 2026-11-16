use super::{NameId, ScopeId, SemanticModel, SlotId};
use crate::types::{TypeId, TypeTable};

impl SemanticModel {
    #[must_use]
    pub fn slot_type(&self, slot: SlotId) -> TypeId {
        self.known_slot_type(slot).unwrap_or(TypeId::UNKNOWN)
    }

    #[must_use]
    pub fn known_slot_type(&self, slot: SlotId) -> Option<TypeId> {
        self.slots[slot.index()]
    }

    pub(crate) fn replace_slot_type(&mut self, slot: SlotId, ty: TypeId) -> bool {
        let is_changed = self.slots[slot.index()] != Some(ty);

        self.slots[slot.index()] = Some(ty);

        is_changed
    }

    pub(crate) fn add_slot(&mut self) -> SlotId {
        let id = SlotId::new(self.slots.len());

        self.slots.push(None);

        id
    }

    pub(crate) fn widen(&mut self, types: &mut TypeTable, slot: SlotId, ty: TypeId) -> bool {
        let current = self.slots[slot.index()];
        let next_type = match current {
            Some(current) => types.union(current, ty),
            None if ty == TypeId::UNKNOWN => return false,
            None => ty,
        };

        self.slots[slot.index()] = Some(next_type);

        current != Some(next_type)
    }

    #[must_use]
    pub fn variable(&self, scope: ScopeId, name: &str) -> Option<SlotId> {
        self.variable_by_id(scope, self.name_id(name)?)
    }

    pub(crate) fn variable_by_id(&self, scope: ScopeId, name: NameId) -> Option<SlotId> {
        self.variables[scope.index()].get(&name).copied()
    }

    pub(crate) fn insert_variable(&mut self, scope: ScopeId, name: &str) -> SlotId {
        let name = self.insert_name(name);

        self.insert_variable_by_id(scope, name)
    }

    pub(crate) fn insert_variable_by_id(&mut self, scope: ScopeId, name: NameId) -> SlotId {
        if let Some(slot) = self.variable_by_id(scope, name) {
            return slot;
        }

        let slot = self.add_slot();

        self.variables[scope.index()].insert(name, slot);

        slot
    }

    pub fn variables(&self, scope: ScopeId) -> impl Iterator<Item = (&str, SlotId)> {
        self.variables[scope.index()]
            .iter()
            .map(|(&name, &slot)| (self.name(name), slot))
    }

    pub(crate) fn variable_count(&self, scope: ScopeId) -> usize {
        self.variables[scope.index()].len()
    }

    #[must_use]
    pub fn outer_scope(&self, scope: ScopeId) -> ScopeId {
        match scope {
            ScopeId::Function(function) => self.function(function).outer_scope,
            ScopeId::Global => ScopeId::Global,
        }
    }

    pub fn visible_scopes(&self, scope: ScopeId) -> impl Iterator<Item = ScopeId> + use<> {
        let outer = self.outer_scope(scope);
        let outer = (outer != scope).then_some(outer);
        let global =
            (scope != ScopeId::Global && outer != Some(ScopeId::Global)).then_some(ScopeId::Global);

        [Some(scope), outer, global].into_iter().flatten()
    }

    #[must_use]
    pub fn resolve_variable(&self, scope: ScopeId, name: &str) -> Option<SlotId> {
        self.visible_variables(scope, self.name_id(name)?).next()
    }

    pub(crate) fn visible_variables(
        &self,
        scope: ScopeId,
        name: NameId,
    ) -> impl Iterator<Item = SlotId> + '_ {
        self.visible_scopes(scope)
            .filter_map(move |scope| self.variable_by_id(scope, name))
    }

    #[must_use]
    pub fn name(&self, id: NameId) -> &str {
        &self.names[id.index()]
    }

    #[must_use]
    pub fn name_id(&self, name: &str) -> Option<NameId> {
        self.name_ids.get(name).copied()
    }

    pub(crate) fn insert_name(&mut self, name: &str) -> NameId {
        if let Some(&id) = self.name_ids.get(name) {
            return id;
        }

        let id = NameId::new(self.names.len());

        self.names.push(name.to_owned());
        self.name_ids.insert(name.to_owned(), id);

        id
    }
}
