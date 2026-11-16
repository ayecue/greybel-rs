use indexmap::IndexMap;

use super::{
    ApiMethod, ApiMethodId, CallContext, CallContextId, DeclaredFunction, DeclaredId,
    DeclaredParameter, FunctionData, FunctionId, GenericInstance, GenericInstanceId, MapOrigin,
    ScopeId, SemanticModel, Site,
};
use crate::types::{TypeId, TypeTable};

impl SemanticModel {
    pub(crate) fn set_guard_type(&mut self, function: FunctionId, ty: TypeId) {
        self.guard_types.insert(function, ty);
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
