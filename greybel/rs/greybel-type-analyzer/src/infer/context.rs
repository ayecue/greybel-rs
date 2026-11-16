use std::mem;

use greybel_core::{Ast, IdentifierKind, MAX_NODE_DEPTH, Node, NodeId, NodeKind, Visitor};
use greybel_symbols::names::{LOCALS, OUTER, SELF, SUPER};

use super::Inference;
use super::call::CallSite;
use crate::analysis::ReferenceTable;
use crate::api::Api;
use crate::model::{CallContext, CallContextId, FunctionId, Parameter, ScopeId, Site};
use crate::types::{FunctionRef, Type, TypeId};

const MAX_CONTEXTS_PER_FUNCTION: usize = 8;
const MAX_CONTEXT_DEPTH: usize = 8;
const MAX_CONTEXT_HEIGHT: usize = MAX_NODE_DEPTH / 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Binding {
    Receiver,
    Parameter(usize),
}

impl Binding {
    fn of(ast: &Ast, parameters: &[Parameter], node: NodeId) -> Option<Self> {
        let NodeKind::Identifier(identifier) = ast.kind(node) else {
            return None;
        };
        let name = ast.resolve(identifier.name);

        match parameters
            .iter()
            .position(|parameter| parameter.name == name)
        {
            Some(0) | None if name == SELF => Some(Self::Receiver),
            Some(position) => Some(Self::Parameter(position)),
            None => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BindingSet {
    receiver: bool,
    parameters: u64,
}

impl BindingSet {
    const ALL_PARAMETERS: u64 = u64::MAX;

    pub(crate) fn contains(self, binding: Binding) -> bool {
        match binding {
            Binding::Receiver => self.receiver,
            Binding::Parameter(position) => {
                position >= 64 || self.parameters & (1 << position) != 0
            }
        }
    }

    fn insert(&mut self, binding: Binding) {
        match binding {
            Binding::Receiver => self.receiver = true,
            Binding::Parameter(position) if position < 64 => self.parameters |= 1 << position,
            Binding::Parameter(_) => {}
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BindingUse {
    pub reads: BindingSet,
    pub assignments: BindingSet,
    pub additions: BindingSet,
    pub shares_locals: bool,
}

struct BindingSearch<'a> {
    ast: &'a Ast,
    api: Api,
    parameters: &'a [Parameter],
    uses: BindingUse,
}

impl BindingSearch<'_> {
    fn add_change(&mut self, node: NodeId) {
        if let Some(binding) = Binding::of(self.ast, self.parameters, node) {
            self.uses.additions.insert(binding);
        }
    }

    fn add_read(&mut self, name: &str) {
        if name == SELF || name == SUPER {
            self.uses.reads.receiver = true;
        }

        if name == LOCALS || name == OUTER {
            self.uses.shares_locals = true;
        }

        if name == LOCALS {
            self.uses.reads.parameters = BindingSet::ALL_PARAMETERS;
        } else if let Some(position) =
            self.parameters.iter().take(64).position(|parameter| {
                parameter.name == name || name == SUPER && parameter.name == SELF
            })
        {
            self.uses.reads.parameters |= 1 << position;
        }
    }
}

impl Visitor for BindingSearch<'_> {
    fn enter(&mut self, _id: NodeId, node: &Node) -> bool {
        match &node.kind {
            NodeKind::Identifier(identifier) if identifier.kind != IdentifierKind::Argument => {
                self.add_read(self.ast.resolve(identifier.name));
            }
            NodeKind::Assignment(assignment) => match self.ast.kind(assignment.variable) {
                NodeKind::Index(index) => self.add_change(index.base),
                NodeKind::Member(member) => self.add_change(member.base),
                _ => {
                    if let Some(binding) =
                        Binding::of(self.ast, self.parameters, assignment.variable)
                    {
                        self.uses.assignments.insert(binding);
                    }
                }
            },
            NodeKind::For(statement) => {
                if let Some(binding) = Binding::of(self.ast, self.parameters, statement.variable) {
                    self.uses.assignments.insert(binding);
                }
            }
            NodeKind::Call(call) => match self.ast.kind(call.base) {
                NodeKind::Member(member) => {
                    if let NodeKind::Identifier(name) = self.ast.kind(member.identifier)
                        && self.api.adds_to_container(self.ast.resolve(name.name))
                    {
                        self.add_change(member.base);
                    }
                }
                NodeKind::Identifier(identifier)
                    if self.api.adds_to_argument(self.ast.resolve(identifier.name)) =>
                {
                    if let Some(&first) = self.ast.list(call.arguments).first() {
                        self.add_change(first);
                    }
                }
                _ => {}
            },
            _ => {}
        }

        true
    }
}

impl Inference<'_> {
    pub(super) fn context_call(
        &mut self,
        function: FunctionId,
        site: &CallSite<'_>,
    ) -> Option<(TypeId, Option<FunctionId>)> {
        let call = self.call_context_of(function, site)?;

        if call.has_unknown() {
            return Some((TypeId::UNKNOWN, None));
        }

        let site = self.model.function(function).site;
        let height = self.documents[site.document as usize]
            .ast
            .nodes()
            .depth(site.node);
        let inferrable = self.context.depth < MAX_CONTEXT_DEPTH
            && self.context.height + height <= MAX_CONTEXT_HEIGHT;
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
            self.context.height += height;
            self.infer_call_context(id);
            self.context.height -= height;
        }

        let context_function = self.context_function(id);
        let returns = self
            .model
            .slot_type(self.model.function(context_function).return_slot);

        Some((returns, Some(context_function)))
    }

    fn call_context_of(
        &mut self,
        function: FunctionId,
        site: &CallSite<'_>,
    ) -> Option<CallContext> {
        let data = self.model.function(function);
        let returns = self.model.slot_type(data.return_slot);
        let returns_fixed =
            returns == TypeId::NULL || data.doc.as_ref().is_some_and(|doc| doc.returns.is_some());
        let uses = self.binding_use(function);
        let changes_depend = self.changes_depend(function, uses, site);

        if returns_fixed && !changes_depend {
            return None;
        }

        let data = self.model.function(function);
        let receiver = site
            .receiver
            .filter(|_| site.method)
            .map(|receiver| receiver.ty);
        let self_parameter = data
            .parameters
            .first()
            .is_some_and(|first| first.name == SELF);
        let mut argument_iter = site.arguments.iter().copied();
        let mut arguments_differ = false;
        let arguments = (0..data.parameters.len())
            .map(|position| {
                let argument = if position == 0 && self_parameter && site.method {
                    receiver
                } else {
                    argument_iter.next()
                };

                if !uses.reads.contains(Binding::Parameter(position)) {
                    return None;
                }

                arguments_differ |= argument != Some(TypeId::ANY);

                argument
            })
            .collect();
        let call = CallContext {
            function,
            receiver: receiver.filter(|_| uses.reads.contains(Binding::Receiver)),
            arguments,
        };
        let own_receiver = self.without_instance_maps(self.model.slot_type(data.receiver_slot));
        let receiver_differs = call
            .self_type(self_parameter)
            .is_some_and(|bound| bound != own_receiver);
        let returns_depend = !returns_fixed
            && self.types.contains(returns, &|shape| {
                matches!(
                    shape,
                    Type::Any | Type::Object(_) | Type::Function(FunctionRef::Body(_))
                )
            });

        (receiver_differs || (returns_depend || changes_depend) && arguments_differ).then_some(call)
    }

    fn changes_depend(&self, function: FunctionId, uses: BindingUse, site: &CallSite<'_>) -> bool {
        let additions = self.additions(function, uses);
        let data = self.model.function(function);
        let is_container = |shape: &Type| matches!(shape, Type::List(_) | Type::Map { .. });
        let is_parameter_container =
            |shape: &Type| is_container(shape) || matches!(shape, Type::Object(_));

        let self_parameter = data
            .parameters
            .first()
            .is_some_and(|first| first.name == SELF);

        (0..=data.parameters.len()).any(|position| {
            let binding = match position.checked_sub(1) {
                None => Binding::Receiver,
                Some(position) => Binding::Parameter(position),
            };
            let is_receiver = match binding {
                Binding::Receiver => true,
                Binding::Parameter(position) => position == 0 && self_parameter,
            };

            additions.contains(binding)
                && !uses.assignments.contains(binding)
                && self
                    .binding_argument(function, binding, site)
                    .is_some_and(|(_, ty)| {
                        if is_receiver {
                            self.types.contains(ty, &is_container)
                        } else {
                            self.types.contains(ty, &is_parameter_container)
                        }
                    })
        })
    }

    pub(super) fn additions(&self, function: FunctionId, uses: BindingUse) -> BindingSet {
        let site = self.model.function(function).site;
        let mut additions = uses.additions;

        if let Some(&indirect) = self.indirect_additions.get(&site) {
            additions.parameters |= indirect.parameters;
            additions.receiver |= indirect.receiver;
        }

        additions
    }

    pub(super) fn add_indirect_addition(&mut self, site: Site, binding: Binding) {
        let indirect = self.indirect_additions.entry(site).or_default();

        if !indirect.contains(binding) {
            indirect.insert(binding);
            self.pass.has_changes = true;
        }
    }

    pub(super) fn binding_argument(
        &self,
        function: FunctionId,
        binding: Binding,
        site: &CallSite<'_>,
    ) -> Option<(NodeId, TypeId)> {
        let data = self.model.function(function);
        let self_parameter = data
            .parameters
            .first()
            .is_some_and(|first| first.name == SELF);
        let receiver = || {
            site.receiver
                .filter(|_| site.method)
                .map(|receiver| (receiver.node, receiver.ty))
        };
        let argument = |index: usize| {
            Some((
                *site.argument_nodes.get(index)?,
                *site.arguments.get(index)?,
            ))
        };

        match binding {
            Binding::Receiver => receiver(),
            Binding::Parameter(0) if self_parameter && site.method => receiver(),
            Binding::Parameter(position) if self_parameter && site.method => argument(position - 1),
            Binding::Parameter(position) => argument(position),
        }
    }

    pub(super) fn binding_use(&mut self, function: FunctionId) -> BindingUse {
        let data = self.model.function(function);

        if let Some(&uses) = self.binding_use.get(&data.site) {
            return uses;
        }

        let ast = self.documents[data.site.document as usize].ast;
        let NodeKind::Function(declaration) = ast.kind(data.site.node) else {
            unreachable!("functions are function nodes")
        };
        let mut search = BindingSearch {
            ast,
            api: self.api,
            parameters: &data.parameters,
            uses: BindingUse::default(),
        };

        for &statement in ast.list(declaration.body) {
            ast.walk(statement, &mut search);
        }

        let uses = search.uses;

        self.binding_use.insert(data.site, uses);

        uses
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
        let references = self.context.spare_references.pop().unwrap_or_else(|| {
            ReferenceTable::new(
                self.documents
                    .iter()
                    .map(|document| document.ast.nodes().len()),
            )
        });
        let outside_references = mem::replace(&mut self.references, references);
        let outside_generics = mem::take(&mut self.type_parameters_in_scope);
        let outside_named_map = self.custom_map.take();
        let outside_context = self.context.current.replace(id);
        let outside_document = mem::replace(&mut self.current_document, site.document);

        self.context.depth += 1;

        self.infer_function_body(site.node);

        self.context.depth -= 1;
        self.current_document = outside_document;
        self.context.current = outside_context;
        self.custom_map = outside_named_map;
        self.type_parameters_in_scope = outside_generics;

        let mut references = mem::replace(&mut self.references, outside_references);

        references.clear();
        self.context.spare_references.push(references);
    }
}
