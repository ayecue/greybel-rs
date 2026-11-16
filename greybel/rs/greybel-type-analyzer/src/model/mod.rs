mod call_context;
mod custom;
mod function;
mod ids;
mod map;
mod signature;
mod slots;

pub use call_context::CallContext;
pub use custom::{CustomType, GenericInstance};
pub use function::{ApiMethod, DeclaredFunction, DeclaredParameter, FunctionData, Parameter};
pub use ids::{
    ApiMethodId, CallContextId, DeclaredId, DocumentIndex, FunctionId, GenericInstanceId, MapId,
    NameId, ScopeId, Site, SlotId,
};
use indexmap::IndexMap;
pub use map::{Key, KeyRef, MapData, MapOrigin};
use meta_utils::BaseType;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
pub use signature::Documentation;
pub(crate) use signature::{FunctionSignature, SignatureParameter, SignatureType};
pub(crate) use slots::SlotList;

use crate::types::{ApiFunction, TypeId, TypeTable};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticModel {
    slots: Vec<Option<TypeId>>,
    maps: Vec<MapData>,
    map_ids: FxHashMap<(MapOrigin, Option<CallContextId>), MapId>,
    functions: Vec<FunctionData>,
    function_ids: FxHashMap<(Site, Option<CallContextId>), FunctionId>,
    variables: Vec<IndexMap<NameId, SlotId, FxBuildHasher>>,
    names: Vec<String>,
    name_ids: FxHashMap<String, NameId>,
    custom_types: IndexMap<String, CustomType, FxBuildHasher>,
    declared_functions: Vec<DeclaredFunction>,
    declared_instance_ids: FxHashMap<(DeclaredId, Vec<TypeId>), DeclaredId>,
    /// The signatures of function types written in annotations, by parameter names and types and return type.
    signature_ids: FxHashMap<(Vec<String>, Vec<TypeId>, TypeId), DeclaredId>,
    api_methods: Vec<ApiMethod>,
    api_method_ids: FxHashMap<ApiMethod, ApiMethodId>,
    generic_instances: Vec<GenericInstance>,
    generic_instance_ids: FxHashMap<GenericInstance, GenericInstanceId>,
    call_contexts: Vec<CallContext>,
    call_context_ids: FxHashMap<CallContext, CallContextId>,
    call_context_counts: FxHashMap<FunctionId, usize>,
    singleton_types: FxHashMap<&'static str, TypeId>,
}

impl Default for SemanticModel {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            maps: Vec::new(),
            map_ids: FxHashMap::default(),
            functions: Vec::new(),
            function_ids: FxHashMap::default(),
            variables: vec![IndexMap::default()],
            names: Vec::new(),
            name_ids: FxHashMap::default(),
            custom_types: IndexMap::default(),
            declared_functions: Vec::new(),
            declared_instance_ids: FxHashMap::default(),
            signature_ids: FxHashMap::default(),
            api_methods: Vec::new(),
            api_method_ids: FxHashMap::default(),
            generic_instances: Vec::new(),
            generic_instance_ids: FxHashMap::default(),
            call_contexts: Vec::new(),
            call_context_ids: FxHashMap::default(),
            call_context_counts: FxHashMap::default(),
            singleton_types: FxHashMap::default(),
        }
    }
}

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

    /// The map of `origin` outside call contexts, if one was inserted.
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

    pub fn functions(&self) -> impl Iterator<Item = FunctionId> + use<'_> {
        (0..self.functions.len())
            .map(FunctionId::new)
            .filter(|&id| self.function(id).context.is_none())
    }

    #[must_use]
    pub fn function(&self, id: FunctionId) -> &FunctionData {
        &self.functions[id.index()]
    }

    pub(crate) fn function_mut(&mut self, id: FunctionId) -> &mut FunctionData {
        &mut self.functions[id.index()]
    }

    #[must_use]
    pub fn function_at(&self, site: Site) -> Option<FunctionId> {
        self.function_in_context(site, None)
    }

    pub(crate) fn function_in_context(
        &self,
        site: Site,
        context: Option<CallContextId>,
    ) -> Option<FunctionId> {
        self.function_ids.get(&(site, context)).copied()
    }

    pub(crate) fn add_function(
        &mut self,
        site: Site,
        outer: ScopeId,
        context: Option<CallContextId>,
    ) -> FunctionId {
        let id = FunctionId::new(self.functions.len());

        self.variables.push(IndexMap::default());

        let returns = self.add_slot();
        let receiver = self.add_slot();

        self.functions.push(FunctionData {
            site,
            context,
            outer_scope: outer,
            parameters: Vec::new(),
            return_slot: returns,
            receiver_slot: receiver,
            doc: None,
        });
        self.function_ids.insert((site, context), id);

        id
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

    #[must_use]
    pub fn declared_function(&self, id: DeclaredId) -> &DeclaredFunction {
        &self.declared_functions[id.index()]
    }

    pub(crate) fn declared_function_mut(&mut self, id: DeclaredId) -> &mut DeclaredFunction {
        &mut self.declared_functions[id.index()]
    }

    pub(crate) fn add_declared_function(&mut self, function: DeclaredFunction) -> DeclaredId {
        let id = DeclaredId::new(self.declared_functions.len());

        self.declared_functions.push(function);

        id
    }

    /// The signature of a function type with these parameters (name and type) and return type, one per distinct
    /// signature.
    pub(crate) fn insert_signature(
        &mut self,
        parameters: Vec<(String, TypeId)>,
        return_type: TypeId,
    ) -> DeclaredId {
        let key = (
            parameters.iter().map(|(name, _)| name.clone()).collect(),
            parameters.iter().map(|&(_, ty)| ty).collect(),
            return_type,
        );

        if let Some(&id) = self.signature_ids.get(&key) {
            return id;
        }

        let id = self.add_declared_function(DeclaredFunction {
            name: String::new(),
            parameters: parameters
                .into_iter()
                .map(|(name, ty)| DeclaredParameter {
                    name,
                    ty,
                    description: None,
                })
                .collect(),
            return_type,
            mutated_parameter: None,
            generic_source: None,
            description: None,
            examples: Vec::new(),
        });

        self.signature_ids.insert(key, id);

        id
    }

    pub(crate) fn insert_declared_instance(
        &mut self,
        generic_source: DeclaredId,
        function: DeclaredFunction,
    ) -> DeclaredId {
        let mut signature: Vec<TypeId> = function
            .parameters
            .iter()
            .map(|parameter| parameter.ty)
            .collect();

        signature.push(function.return_type);

        let key = (generic_source, signature);

        if let Some(&id) = self.declared_instance_ids.get(&key) {
            return id;
        }

        let id = self.add_declared_function(function);

        self.declared_instance_ids.insert(key, id);

        id
    }

    #[must_use]
    pub fn api_method(&self, id: ApiMethodId) -> &ApiMethod {
        &self.api_methods[id.index()]
    }

    pub(crate) fn insert_api_method(&mut self, method: ApiMethod) -> ApiMethodId {
        if let Some(&id) = self.api_method_ids.get(&method) {
            return id;
        }

        let id = ApiMethodId::new(self.api_methods.len());

        self.api_methods.push(method.clone());
        self.api_method_ids.insert(method, id);

        id
    }

    #[must_use]
    pub fn generic_instance(&self, id: GenericInstanceId) -> &GenericInstance {
        &self.generic_instances[id.index()]
    }

    pub(crate) fn insert_generic_instance(
        &mut self,
        specialization: GenericInstance,
    ) -> GenericInstanceId {
        if let Some(&id) = self.generic_instance_ids.get(&specialization) {
            return id;
        }

        let id = GenericInstanceId::new(self.generic_instances.len());

        self.generic_instances.push(specialization.clone());
        self.generic_instance_ids.insert(specialization, id);

        id
    }

    #[must_use]
    pub fn call_context(&self, id: CallContextId) -> &CallContext {
        &self.call_contexts[id.index()]
    }

    pub(crate) fn bound_arguments(&self, function: FunctionId) -> Option<&[Option<TypeId>]> {
        let data = self.function(function);
        let context = self.call_context(data.context?);

        (self.function(context.function).site == data.site).then_some(&context.arguments[..])
    }

    pub(crate) fn call_context_id(&self, context: &CallContext) -> Option<CallContextId> {
        self.call_context_ids.get(context).copied()
    }

    pub(crate) fn call_context_count(&self, function: FunctionId) -> usize {
        self.call_context_counts
            .get(&function)
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn add_call_context(&mut self, context: CallContext) -> CallContextId {
        let id = CallContextId::new(self.call_contexts.len());

        *self
            .call_context_counts
            .entry(context.function)
            .or_default() += 1;
        self.call_contexts.push(context.clone());
        self.call_context_ids.insert(context, id);

        id
    }

    #[must_use]
    pub fn generic_instance_of(&self, types: &TypeTable, ty: TypeId) -> Option<&GenericInstance> {
        let map = types.object_map(ty)?;

        match self.map(map).origin {
            MapOrigin::GenericInstance(id) => Some(self.generic_instance(id)),
            _ => None,
        }
    }
}
