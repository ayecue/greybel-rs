use std::sync::Arc;

use greybel_core::NodeId;
use greybel_symbols::names::SELF;

use super::Inference;
use super::generic::Evidence;
use crate::model::SignatureType;
use crate::types::{FunctionRef, Type, TypeId};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Receiver {
    pub node: NodeId,
    pub ty: TypeId,
}

pub(crate) struct CallSite<'s> {
    pub method: bool,
    pub receiver: Option<Receiver>,
    pub arguments: &'s [TypeId],
    pub argument_nodes: &'s [NodeId],
}

impl CallSite<'_> {
    pub(crate) const PLAIN: CallSite<'static> = CallSite {
        method: false,
        receiver: None,
        arguments: &[],
        argument_nodes: &[],
    };

    pub(crate) fn member_access(receiver: Option<Receiver>) -> CallSite<'static> {
        CallSite {
            method: true,
            receiver,
            arguments: &[],
            argument_nodes: &[],
        }
    }
}

impl Inference<'_> {
    pub(crate) fn call_result_type(&mut self, callee: TypeId, site: &CallSite<'_>) -> TypeId {
        self.map_variants(callee, |this, variant| match *this.types.get(variant) {
            Type::Function(function) => this.call_function(function, site),
            _ => variant,
        })
    }

    pub(crate) fn map_variants(
        &mut self,
        ty: TypeId,
        mut each: impl FnMut(&mut Self, TypeId) -> TypeId,
    ) -> TypeId {
        if !self.types.is_union(ty) {
            return each(self, ty);
        }

        let variants: Vec<TypeId> = self.types.variants(ty).collect();
        let results: Vec<TypeId> = variants
            .into_iter()
            .map(|variant| each(self, variant))
            .collect();

        self.types.union_all(results)
    }

    fn call_function(&mut self, function: FunctionRef, site: &CallSite<'_>) -> TypeId {
        let function = match function {
            FunctionRef::ApiMethod(id) => FunctionRef::Api(self.model.api_method(id).function),
            _ => function,
        };
        let in_context = match function {
            FunctionRef::Body(id) => self.context_call(id, site),
            _ => None,
        };
        let returns = match in_context {
            Some((returns, _)) => returns,
            None => self.lookup().return_type(function),
        };

        self.apply_call_side_effect(function);

        let mutation = self.call_mutation(function, site);

        if let FunctionRef::Body(id) = function
            && self
                .model
                .function(id)
                .doc
                .as_ref()
                .is_none_or(|doc| doc.mutated_parameter.is_none())
        {
            let body = in_context.and_then(|(_, context)| context).unwrap_or(id);

            self.apply_binding_changes(id, body, site);
        }

        if mutation.is_none() && !self.has_type_parameters(returns) {
            return returns;
        }

        let parameters = self.parameter_types(function, site.method);
        let mut evidence = Evidence::default();

        for (&parameter, &argument) in parameters.iter().zip(site.arguments) {
            self.bind(parameter, argument, &mut evidence);
        }

        let mut receiver = site.receiver.map(|receiver| receiver.ty);

        if let Some(mutation) = mutation
            && let Some(mutation_type) = self.apply_mutation(&mutation, &evidence)
            && mutation.targets_receiver()
        {
            receiver = receiver.map(|receiver| self.types.union(receiver, mutation_type));
        }

        if let FunctionRef::Api(api) = function
            && let (Some(site_receiver), Some(pattern)) =
                (site.receiver, self.lookup().api_receiver_type(api))
        {
            self.bind(pattern, site_receiver.ty, &mut evidence);
        }

        if let Some(receiver) = receiver {
            evidence.entry(SELF.into()).or_default().push(receiver);
        }

        let mut names = Vec::new();

        self.type_parameter_names(returns, &mut names);

        let bindings = names
            .into_iter()
            .map(|name| {
                let bound = evidence
                    .get(&name)
                    .map_or(TypeId::UNKNOWN, |types| self.types.union_all(types.clone()));

                (name, bound)
            })
            .collect();

        self.substitute(returns, &bindings)
    }

    pub(super) fn parameter_types(&mut self, function: FunctionRef, method: bool) -> Arc<[TypeId]> {
        match function {
            FunctionRef::Any => Arc::new([]),
            FunctionRef::Api(api) => self.lookup().api_parameter_types(api),
            FunctionRef::ApiMethod(id) => self
                .model
                .api_method(id)
                .parameters
                .iter()
                .copied()
                .collect(),
            FunctionRef::Signature(id) => {
                let original = self
                    .model
                    .declared_function(id)
                    .generic_source
                    .unwrap_or(id);

                self.model
                    .declared_function(original)
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty)
                    .collect()
            }
            FunctionRef::Body(_) => self
                .model
                .signature(function, method)
                .map(|signature| {
                    signature
                        .parameters
                        .iter()
                        .map(|parameter| match parameter.ty {
                            SignatureType::Value(ty) => ty,
                            SignatureType::Api(_) => TypeId::ANY,
                            SignatureType::Guard { .. } => TypeId::NUMBER,
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}
