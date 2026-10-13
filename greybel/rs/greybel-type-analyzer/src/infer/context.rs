use std::mem;

use greybel_core::MAX_NODE_DEPTH;
use greybel_symbols::names::SELF;

use super::Inference;
use super::call::CallSite;
use crate::analysis::ReferenceTable;
use crate::model::{CallContext, CallContextId, FunctionId, Parameter, ScopeId};
use crate::types::{FunctionRef, Type, TypeId};

const MAX_CONTEXTS_PER_FUNCTION: usize = 8;
const MAX_CONTEXT_DEPTH: usize = 8;
const MAX_CONTEXT_HEIGHT: usize = MAX_NODE_DEPTH / 2;

impl Inference<'_> {
    pub(super) fn context_return_type(
        &mut self,
        function: FunctionId,
        site: &CallSite<'_>,
    ) -> Option<TypeId> {
        let call = self.call_context_of(function, site)?;

        if call.has_unknown() {
            return Some(TypeId::UNKNOWN);
        }

        let site = self.model.function(function).site;
        let height = self.documents[site.document as usize]
            .ast
            .nodes()
            .depth(site.node);
        let inferrable = self.context_depth < MAX_CONTEXT_DEPTH
            && self.context_height + height <= MAX_CONTEXT_HEIGHT;
        let key = self.context_key(&call);
        let id = match self.model.call_context_id(&key) {
            Some(id) => id,
            None if inferrable
                && self.model.call_context_count(function) < MAX_CONTEXTS_PER_FUNCTION =>
            {
                self.add_call_context(key)
            }
            None => return None,
        };

        self.bind_arguments(id, &call);
        if inferrable && self.pass.inferred_contexts.insert(id) {
            self.context_height += height;
            self.infer_call_context(id);
            self.context_height -= height;
        }

        let context_function = self.context_function(id);

        Some(
            self.model
                .slot_type(self.model.function(context_function).return_slot),
        )
    }

    fn call_context_of(&self, function: FunctionId, site: &CallSite<'_>) -> Option<CallContext> {
        let data = self.model.function(function);
        let returns = self.model.slot_type(data.return_slot);

        if returns == TypeId::NULL || data.doc.as_ref().is_some_and(|doc| doc.returns.is_some()) {
            return None;
        }

        let receiver = site
            .receiver
            .filter(|_| site.method)
            .map(|receiver| receiver.ty);
        let self_parameter = data
            .parameters
            .first()
            .is_some_and(|first| first.name == SELF);
        let mut argument_iter = site.arguments.iter().copied();
        let arguments = (0..data.parameters.len())
            .map(|position| {
                if position == 0 && self_parameter && site.method {
                    receiver
                } else {
                    argument_iter.next()
                }
            })
            .collect();
        let call = CallContext {
            function,
            receiver,
            arguments,
        };
        let receiver_differs = call
            .self_type(self_parameter)
            .is_some_and(|bound| bound != self.model.slot_type(data.receiver_slot));
        let returns_depend = self.types.contains(returns, &|shape| {
            matches!(
                shape,
                Type::Any | Type::Unknown | Type::Object(_) | Type::Function(FunctionRef::Body(_))
            )
        });
        let arguments_differ = call
            .arguments
            .iter()
            .any(|&argument| argument != Some(TypeId::ANY));

        (receiver_differs || returns_depend && arguments_differ).then_some(call)
    }

    fn context_key(&mut self, call: &CallContext) -> CallContext {
        CallContext {
            function: call.function,
            receiver: call
                .receiver
                .map(|receiver| self.context_key_type(receiver)),
            arguments: call
                .arguments
                .iter()
                .map(|argument| argument.map(|argument| self.context_key_type(argument)))
                .collect(),
        }
    }

    fn context_key_type(&mut self, ty: TypeId) -> TypeId {
        let ty = self.creation_site_type(ty);
        let present: Vec<TypeId> = self
            .types
            .variants(ty)
            .filter(|&variant| variant != TypeId::NULL)
            .collect();

        if present.is_empty() {
            ty
        } else {
            self.types.union_all(present)
        }
    }

    fn creation_site_type(&mut self, ty: TypeId) -> TypeId {
        let is_object = |shape: &Type| matches!(shape, Type::Object(_));

        if !self.types.contains(ty, &is_object) {
            return ty;
        }

        match self.types.get(ty).clone() {
            Type::Object(map) => {
                let data = self.model.map(map);

                if data.context.is_none() {
                    return ty;
                }

                let site_map = self.model.insert_map(data.origin);

                self.object_type(site_map)
            }
            Type::List(item) => {
                let item = self.creation_site_type(item);

                self.types.list(item)
            }
            Type::Map { key, value } => {
                let (key, value) = (self.creation_site_type(key), self.creation_site_type(value));

                self.types.map(key, value)
            }
            Type::Union(variants) => {
                let variants: Vec<TypeId> = variants
                    .iter()
                    .map(|&variant| self.creation_site_type(variant))
                    .collect();

                self.types.union_all(variants)
            }
            _ => ty,
        }
    }

    fn add_call_context(&mut self, key: CallContext) -> CallContextId {
        let callee = self.model.function(key.function).clone();
        let id = self.model.add_call_context(key);
        let context_function = self
            .model
            .add_function(callee.site, callee.outer_scope, Some(id));
        let scope = ScopeId::Function(context_function);
        let parameters = callee
            .parameters
            .iter()
            .map(|parameter| Parameter {
                slot: self.model.insert_variable(scope, &parameter.name),
                ..parameter.clone()
            })
            .collect();
        let data = self.model.function_mut(context_function);

        data.parameters = parameters;
        data.doc = callee.doc;

        let current = mem::replace(&mut self.current_document, callee.site.document);

        self.declare_variables(scope, callee.site.node);
        self.current_document = current;

        id
    }

    fn bind_arguments(&mut self, id: CallContextId, call: &CallContext) {
        let callee = self.model.function(call.function);
        let receiver = call
            .receiver
            .unwrap_or_else(|| self.model.slot_type(callee.receiver_slot));
        let context_function = self.model.function(self.context_function(id));
        let receiver_slot = context_function.receiver_slot;
        let bound: Vec<_> = context_function
            .parameters
            .iter()
            .zip(&call.arguments)
            .filter_map(|(parameter, &argument)| {
                let is_annotated = context_function
                    .doc
                    .as_ref()
                    .and_then(|doc| doc.param(&parameter.name))
                    .is_some_and(|param| param.ty.is_some());

                Some((parameter.slot, argument.filter(|_| !is_annotated)?))
            })
            .collect();

        self.widen(receiver_slot, receiver);
        for (slot, argument) in bound {
            self.widen(slot, argument);
        }
    }

    fn context_function(&self, id: CallContextId) -> FunctionId {
        let site = self
            .model
            .function(self.model.call_context(id).function)
            .site;

        self.model
            .function_in_context(site, Some(id))
            .expect("added with its call context")
    }

    fn infer_call_context(&mut self, id: CallContextId) {
        let site = self
            .model
            .function(self.model.call_context(id).function)
            .site;
        let references = self.context_references.pop().unwrap_or_else(|| {
            ReferenceTable::new(
                self.documents
                    .iter()
                    .map(|document| document.ast.nodes().len()),
            )
        });
        let outside_references = mem::replace(&mut self.references, references);
        let outside_types = mem::take(&mut self.context_types);
        let outside_narrow_stack = mem::take(&mut self.narrow_stack);
        let outside_generics = mem::take(&mut self.type_parameters_in_scope);
        let outside_named_map = self.custom_map.take();
        let outside_context = self.call_context.replace(id);
        let outside_document = mem::replace(&mut self.current_document, site.document);

        self.context_depth += 1;

        self.infer_function_body(site.node);

        self.context_depth -= 1;
        self.current_document = outside_document;
        self.call_context = outside_context;
        self.custom_map = outside_named_map;
        self.type_parameters_in_scope = outside_generics;
        self.narrow_stack = outside_narrow_stack;
        self.context_types = outside_types;

        let mut references = mem::replace(&mut self.references, outside_references);

        references.clear();
        self.context_references.push(references);
    }
}
