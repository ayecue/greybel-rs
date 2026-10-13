use greybel_core::ast::NodeFieldContainer;
use greybel_core::{NodeId, NodeKind, UnaryOp};
use greybel_symbols::annotations::Tag;
use greybel_symbols::names::{ENTRY_KEY, ENTRY_VALUE, LOOP_INDEX_PREFIX, LOOP_INDEX_SUFFIX, SELF};

use super::Inference;
use super::narrow::FactTable;
use crate::annotation::TypeExpr;
use crate::model::{FunctionId, Key, KeyRef, MapId, MapOrigin, ScopeId, Site, SlotId};
use crate::types::{Type, TypeId};

impl Inference<'_> {
    pub(crate) fn infer_statement(&mut self, node: NodeId) {
        let ast = self.ast();

        match ast.kind(node) {
            NodeKind::Assignment(assignment) => {
                self.validate_self_assignment(assignment.variable, assignment.init);
                self.custom_map = self.custom_map_at(node, assignment.init);

                let mut value = self.infer_value(assignment.init);

                self.custom_map = None;
                if let Some(annotation_type) = self.annotated_type_at(node) {
                    value = annotation_type;
                }

                self.assign(assignment.variable, value, assignment.init);
            }
            NodeKind::CallStatement(statement) => {
                self.infer_value(statement.expression);
            }
            NodeKind::Return(statement) => {
                let value = match statement.argument {
                    Some(argument) => self.infer_value(argument),
                    None => TypeId::NULL,
                };

                if let ScopeId::Function(function) = self.current_scope {
                    let data = self.model.function(function);
                    let is_annotated = data.doc.as_ref().is_some_and(|doc| doc.returns.is_some());
                    let (slot, site) = (data.return_slot, data.site);

                    if is_annotated {
                        let name = self.function_name(site.node);
                        let annotation_type = self.model.slot_type(slot);

                        self.validate_against_annotation(
                            node,
                            Tag::Returns,
                            &name,
                            annotation_type,
                            value,
                        );
                    } else {
                        self.widen(slot, value);
                    }
                }
            }
            NodeKind::If(statement) => {
                let mut previous_false = FactTable::default();

                for &clause in ast.list(statement.clauses) {
                    match ast.kind(clause) {
                        NodeKind::IfClause(clause) => {
                            self.infer_value(clause.condition);
                            self.validate_condition(clause.condition);

                            let guard = self.condition_guard(clause.condition);
                            let facts = previous_false.intersection(&guard.when_true);

                            self.infer_block_with_facts(&facts, ast.list(clause.body));
                            previous_false = previous_false.intersection(&guard.when_false);
                        }
                        NodeKind::ElseClause(clause) => {
                            self.infer_block_with_facts(&previous_false, ast.list(clause.body));
                        }
                        _ => {}
                    }
                }
            }
            NodeKind::While(statement) => {
                self.infer_value(statement.condition);
                self.enter_loop(node);
                self.infer_block(ast.list(statement.body));
                self.exit_loop();
            }
            NodeKind::For(statement) => {
                let iterable = self.infer_value(statement.sequence);
                let item = self.iteration_item_type(node, iterable);

                self.assign(statement.variable, item, statement.sequence);
                if let Some(name) = self.identifier_name(statement.variable) {
                    let index = self.model.insert_variable(
                        self.current_scope,
                        &format!("{LOOP_INDEX_PREFIX}{name}{LOOP_INDEX_SUFFIX}"),
                    );

                    self.widen(index, TypeId::NUMBER);
                }

                self.enter_loop(node);
                self.infer_block(ast.list(statement.body));
                self.exit_loop();
            }
            NodeKind::Break(_)
            | NodeKind::Continue(_)
            | NodeKind::Noop(_)
            | NodeKind::Invalid(_) => {}
            NodeKind::Import(_)
            | NodeKind::Include(_)
            | NodeKind::ImportCode(_)
            | NodeKind::Envar(_)
            | NodeKind::Inject(_)
            | NodeKind::File(_)
            | NodeKind::Line(_)
            | NodeKind::Debugger(_) => {
                self.infer_directive(node);
            }
            _ => {
                self.infer_value(node);
            }
        }
    }

    pub(crate) fn assign(&mut self, target: NodeId, value: TypeId, init: NodeId) {
        let ast = self.ast();

        match ast.kind(target) {
            NodeKind::Identifier(identifier) => {
                let slot = self
                    .model
                    .insert_variable(self.current_scope, ast.resolve(identifier.name));

                self.assign_variable(target, slot, value);
            }
            NodeKind::Member(member) => {
                let name = self.identifier_name(member.identifier).unwrap_or_default();

                if let Some(scope) = self.scope_for_name(member.base) {
                    let slot = self.model.insert_variable(scope, name);

                    self.assign_variable(member.identifier, slot, value);

                    let ty = self.model.slot_type(slot);

                    self.record_node_type(target, ty);

                    return;
                }

                let base = self.infer_value(member.base);
                let key = KeyRef::String(name);

                self.assign_entry(base, key, value);

                let slots = self.lookup().entry_slots(base, key);

                self.add_definition(member.identifier, &slots);
                self.bind_receiver(init, base);

                let ty = self.lookup().member_type(base, key);

                self.record_node_type(member.identifier, ty);
                self.record_node_type(target, ty);
            }
            NodeKind::Index(index) => {
                let base = self.infer_value(index.base);
                let key = match self.literal_key(index.index) {
                    Some(key) => {
                        self.infer_value(index.index);
                        self.assign_entry(base, key.as_key_ref(), value);

                        let slots = self.lookup().entry_slots(base, key.as_key_ref());

                        self.add_definition(index.index, &slots);
                        self.bind_receiver(init, base);

                        match key {
                            Key::String(_) => TypeId::STRING,
                            Key::Number(_) => TypeId::NUMBER,
                        }
                    }
                    None => {
                        let key = self.infer_value(index.index);

                        self.assign_dynamic_entry(base, key, value);

                        key
                    }
                };

                self.assign_into_containers(index.base, base, key, value);
                self.record_node_type(target, value);
            }
            _ => {
                self.infer_value(target);
            }
        }
    }

    fn assign_variable(&mut self, node: NodeId, slot: SlotId, value: TypeId) {
        self.widen(slot, value);
        self.narrow_stack.clear_facts(slot);
        self.add_definition(node, &[slot]);

        let ty = self.model.slot_type(slot);

        self.record_node_type(node, ty);
    }

    fn bind_receiver(&mut self, init: NodeId, receiver: TypeId) {
        if let NodeKind::Function(_) = self.ast().kind(init) {
            let function = self.function_id(init);
            let slot = self.model.function(function).receiver_slot;

            self.widen(slot, receiver);
        }
    }

    pub(crate) fn infer_function_body(&mut self, node: NodeId) -> FunctionId {
        let ast = self.ast();
        let function = self.function_id(node);
        let NodeKind::Function(declaration) = ast.kind(node) else {
            unreachable!("functions are function nodes")
        };
        let previous = std::mem::replace(&mut self.current_scope, ScopeId::Function(function));
        let data = self.model.function(function);
        let (receiver, returns) = (data.receiver_slot, data.return_slot);
        let doc = data.doc.clone();
        let parameters: Vec<_> = data
            .parameters
            .iter()
            .map(|parameter| {
                let annotation_type = doc
                    .as_ref()
                    .and_then(|doc| doc.param(&parameter.name)?.ty.clone());

                (
                    parameter.node,
                    parameter.slot,
                    parameter.name == SELF,
                    annotation_type,
                )
            })
            .collect();
        let arguments = self.model.bound_arguments(function).map(<[_]>::to_vec);
        let depth = self.type_parameters_in_scope.len();

        if let Some(doc) = &doc {
            let templates = self.template_parameters(&doc.templates);

            self.type_parameters_in_scope.extend(templates);
        }

        for (position, (parameter, slot, is_self, annotation_type)) in
            parameters.into_iter().enumerate()
        {
            let argument = arguments
                .as_ref()
                .and_then(|arguments| arguments.get(position).copied().flatten());
            let inferred_type = match (argument, ast.kind(parameter)) {
                (Some(_), _) => self.model.slot_type(slot),
                (None, NodeKind::Assignment(assignment)) => self.infer_value(assignment.init),
                (None, _) if position == 0 && is_self => self.model.slot_type(receiver),
                (None, _) if arguments.is_some() => TypeId::NULL,
                (None, _) => TypeId::ANY,
            };
            let annotation_type = annotation_type
                .map(|annotation_type| self.resolve_signature_type(&annotation_type));
            let value = match annotation_type {
                Some(annotation_type) if annotation_type != TypeId::UNKNOWN => {
                    if let NodeKind::Assignment(assignment) = ast.kind(parameter)
                        && let Some(name) = self.identifier_name(assignment.variable)
                    {
                        self.validate_against_annotation(
                            parameter,
                            Tag::Param,
                            name,
                            annotation_type,
                            inferred_type,
                        );
                    }

                    annotation_type
                }
                _ => inferred_type,
            };

            self.widen(slot, value);

            let ty = self.model.slot_type(slot);
            let variable = match ast.kind(parameter) {
                NodeKind::Assignment(assignment) => assignment.variable,
                _ => parameter,
            };

            self.add_definition(variable, &[slot]);
            if let NodeKind::Assignment(assignment) = ast.kind(parameter) {
                self.record_node_type(assignment.variable, ty);
            }

            self.record_node_type(parameter, ty);
        }

        self.infer_block(ast.list(declaration.body));

        let annotated_returns = doc.and_then(|doc| doc.returns);
        let returns_nothing = ast
            .kind(node)
            .as_scope()
            .is_none_or(|scope| scope.scope().returns.is_empty());

        if let Some(annotation_type) = annotated_returns {
            let annotation_type = self.resolve_signature_type(&annotation_type);

            self.widen(returns, annotation_type);
        } else if returns_nothing {
            self.widen(returns, TypeId::NULL);
        }

        self.type_parameters_in_scope.truncate(depth);
        self.current_scope = previous;

        function
    }

    fn resolve_signature_type(&mut self, expr: &TypeExpr) -> TypeId {
        self.implicit_generics = true;

        let ty = self.resolve_type(expr);

        self.implicit_generics = false;

        ty
    }

    fn custom_map_at(&mut self, node: NodeId, init: NodeId) -> Option<(Site, MapId)> {
        let makes_map = match self.ast().kind(init) {
            NodeKind::Map(_) => true,
            NodeKind::Unary(unary) => unary.operator == UnaryOp::New,
            _ => false,
        };

        if !makes_map {
            return None;
        }

        let named_type = self.custom_type_at(node)?;

        Some((self.site(init), self.types.object_map(named_type)?))
    }

    fn iteration_item_type(&mut self, node: NodeId, iterable: TypeId) -> TypeId {
        let iterable = self.types.erase_type_parameters(iterable);
        let variants: Vec<TypeId> = self.types.variants(iterable).collect();
        let items: Vec<TypeId> = variants
            .into_iter()
            .map(|variant| match self.types.get(variant).clone() {
                Type::List(item) => item,
                Type::String => TypeId::STRING,
                Type::Any => TypeId::ANY,
                Type::Map { key, value } => self.key_value_pair_type(node, key, value),
                Type::Object(map) => {
                    let (key, value) = self.entry_types(map);

                    self.key_value_pair_type(node, key, value)
                }
                Type::Class(_) => self.key_value_pair_type(node, TypeId::ANY, TypeId::ANY),
                _ => TypeId::UNKNOWN,
            })
            .collect();

        self.types.union_all(items)
    }

    fn key_value_pair_type(&mut self, node: NodeId, key: TypeId, value: TypeId) -> TypeId {
        let map = self
            .model
            .insert_map_in(MapOrigin::Iteration(self.site(node)), self.call_context);
        let object = self.object_type(map);

        self.assign_entry(object, KeyRef::String(ENTRY_KEY), key);
        self.assign_entry(object, KeyRef::String(ENTRY_VALUE), value);

        object
    }
}
