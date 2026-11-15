use greybel_core::{BinaryOp, Dialect, LiteralValue, NodeId, NodeKind, UnaryOp};
use greybel_symbols::names::{GLOBALS, LOCALS, MODULE, OUTER, PARAMS, SCOPES, SELF, SUPER};
use greybel_symbols::types::{FUNC_REF, LIST, MAP, NUMBER, STRING};
use meta_utils::BaseType;

use super::Inference;
use super::call::{CallSite, Receiver};
use crate::diagnostic::DiagnosticKind;
use crate::model::{Key, KeyRef, MapId, MapOrigin, ScopeId};
use crate::types::{FunctionRef, Type, TypeId};

impl Inference<'_> {
    pub(crate) fn infer_value(&mut self, node: NodeId) -> TypeId {
        match self.ast().kind(node) {
            NodeKind::Identifier(_) => {
                let target = self.infer_reference(node);

                self.call_result_type(target, &CallSite::PLAIN)
            }
            NodeKind::Member(_) => {
                let (target, receiver) = self.infer_reference_with_receiver(node);

                self.call_result_type(target, &CallSite::member_access(receiver))
            }
            NodeKind::Index(index) => {
                let (target, receiver) = self.infer_reference_with_receiver(node);

                if index.is_statement_start {
                    self.call_result_type(target, &CallSite::member_access(receiver))
                } else {
                    target
                }
            }
            NodeKind::Parenthesis(parenthesis) => {
                let ty = self.infer_value(parenthesis.expression);

                self.record_node_type(node, ty);

                ty
            }
            _ => self.infer_expression(node),
        }
    }

    pub(crate) fn infer_reference(&mut self, node: NodeId) -> TypeId {
        self.infer_reference_with_receiver(node).0
    }

    fn infer_reference_with_receiver(&mut self, node: NodeId) -> (TypeId, Option<Receiver>) {
        let ast = self.ast();
        let mut receiver = None;
        let ty = match ast.kind(node) {
            NodeKind::Identifier(identifier) => {
                self.infer_identifier(node, ast.resolve(identifier.name))
            }
            NodeKind::Member(member) => {
                let name = self.identifier_name(member.identifier).unwrap_or_default();
                let ty = match self.scope_for_name(member.base) {
                    Some(scope) => {
                        let scope_map = self.scope_map_type();

                        self.record_node_type(member.base, scope_map);

                        match self.model.variable(scope, name) {
                            Some(slot) => {
                                self.add_reference(member.identifier, &[slot]);

                                self.model.slot_type(slot)
                            }
                            None => {
                                receiver = Some(Receiver {
                                    node: member.base,
                                    ty: scope_map,
                                });

                                self.lookup().member_type(scope_map, KeyRef::String(name))
                            }
                        }
                    }
                    None => {
                        let base = self.infer_value(member.base);

                        receiver = Some(Receiver {
                            node: member.base,
                            ty: base,
                        });

                        let key = KeyRef::String(name);
                        let slots = self.lookup().entry_slots(base, key);

                        self.add_reference(member.identifier, &slots);

                        self.lookup().member_type(base, key)
                    }
                };
                let method = match receiver {
                    Some(receiver) => self.method_type(ty, receiver.ty),
                    None => ty,
                };

                self.record_node_type(member.identifier, method);
                self.record_node_type(node, method);

                return (ty, receiver);
            }
            NodeKind::Index(index) => {
                let base = self.infer_value(index.base);

                receiver = Some(Receiver {
                    node: index.base,
                    ty: base,
                });
                self.validate_index(node, base);

                match self.literal_key(index.index) {
                    Some(key) => {
                        self.infer_value(index.index);

                        self.index_type(base, Some(key.as_key_ref()), TypeId::UNKNOWN)
                    }
                    None => {
                        let key = self.infer_value(index.index);

                        self.index_type(base, None, key)
                    }
                }
            }
            NodeKind::Parenthesis(parenthesis) => {
                return self.infer_reference_with_receiver(parenthesis.expression);
            }
            _ => return (self.infer_expression(node), None),
        };

        self.record_node_type(node, ty);

        (ty, receiver)
    }

    fn infer_call(&mut self, node: NodeId) -> TypeId {
        let ast = self.ast();
        let NodeKind::Call(call) = ast.kind(node) else {
            unreachable!()
        };
        let (callee, receiver) = self.infer_reference_with_receiver(call.base);
        let mut arguments = self.type_buffers.take();

        arguments.extend(
            ast.list(call.arguments)
                .iter()
                .map(|&argument| self.infer_value(argument)),
        );

        let method = matches!(
            ast.kind(call.base),
            NodeKind::Member(_) | NodeKind::Index(_)
        );

        self.validate_call(node, callee, &arguments, method);

        let site = CallSite {
            method,
            receiver,
            arguments: &arguments,
            argument_nodes: ast.list(call.arguments),
        };
        let result = self.call_result_type(callee, &site);

        self.type_buffers.release(arguments);

        result
    }

    fn infer_list(&mut self, node: NodeId) -> TypeId {
        let ast = self.ast();
        let NodeKind::List(list) = ast.kind(node) else {
            unreachable!()
        };
        let mut items = self.type_buffers.take();

        items.extend(
            ast.list(list.fields)
                .iter()
                .map(|&item| self.infer_value(item)),
        );

        let item = self.types.union_all(items.iter().copied());

        self.type_buffers.release(items);

        self.types.list(item)
    }

    fn infer_map_literal(&mut self, node: NodeId) -> TypeId {
        let ast = self.ast();
        let NodeKind::Map(map) = ast.kind(node) else {
            unreachable!()
        };
        let id = self.resolve_map(node, MapOrigin::Literal(self.site(node)));
        let object = self.object_type(id);
        let mut literal_keys = Vec::new();

        for &entry in ast.list(map.fields) {
            let NodeKind::MapEntry(field) = ast.kind(entry) else {
                continue;
            };
            let value = self.infer_value(field.value);

            match self.literal_key(field.key) {
                Some(key) => {
                    let duplicate = literal_keys.contains(&key);

                    if duplicate {
                        let text = ast.text(field.key).unwrap_or_default().to_owned();

                        self.report(
                            DiagnosticKind::DuplicateKey,
                            field.key,
                            format!("Duplicate map literal key: {text}"),
                        );
                    }

                    self.infer_value(field.key);
                    self.assign_entry(object, key.as_key_ref(), value);

                    let slots = self.lookup().entry_slots(object, key.as_key_ref());

                    self.add_definition(field.key, &slots);
                    if !duplicate {
                        literal_keys.push(key);
                    }
                }
                None => {
                    let key = self.infer_value(field.key);

                    self.assign_dynamic_entry(object, key, value);
                }
            }

            self.record_node_type(entry, value);
        }

        object
    }

    fn infer_slice(&mut self, node: NodeId) -> TypeId {
        let ast = self.ast();
        let NodeKind::Slice(slice) = ast.kind(node) else {
            unreachable!()
        };
        let base = self.infer_value(slice.base);
        let base = self.types.erase_type_parameters(base);

        self.infer_value(slice.left);
        self.infer_value(slice.right);

        let variants: Vec<TypeId> = self
            .types
            .variants(base)
            .filter(|&variant| {
                matches!(
                    self.types.get(variant),
                    Type::String | Type::List(_) | Type::Any
                )
            })
            .collect();

        self.types.union_all(variants)
    }

    fn infer_unary(&mut self, node: NodeId) -> TypeId {
        let NodeKind::Unary(unary) = self.ast().kind(node) else {
            unreachable!()
        };

        match unary.operator {
            UnaryOp::AddressOf => match self.infer_reference_with_receiver(unary.argument) {
                (ty, Some(receiver)) => self.method_type(ty, receiver.ty),
                (ty, None) => ty,
            },
            UnaryOp::New => {
                let parent = self.infer_value(unary.argument);
                let map = self.resolve_map(node, MapOrigin::New(self.site(node)));
                let slot = self.model.map(map).isa;

                self.widen(slot, parent);

                self.object_type(map)
            }
            UnaryOp::Not | UnaryOp::Negate => {
                let operand = self.infer_value(unary.argument);

                Self::propagate_unknown(&[operand], TypeId::NUMBER)
            }
        }
    }

    fn infer_expression(&mut self, node: NodeId) -> TypeId {
        let ast = self.ast();
        let ty = match ast.kind(node) {
            NodeKind::Literal(literal) => match literal.value {
                LiteralValue::String(_) => TypeId::STRING,
                LiteralValue::Number(_) | LiteralValue::Boolean(_) => TypeId::NUMBER,
                LiteralValue::Nil => TypeId::NULL,
            },
            NodeKind::Function(_) => {
                let function = self.infer_function_body(node);

                self.types.function(FunctionRef::Body(function))
            }
            NodeKind::Call(_) => self.infer_call(node),
            NodeKind::List(_) => self.infer_list(node),
            NodeKind::ListItem(item) => self.infer_value(item.value),
            NodeKind::Map(_) => self.infer_map_literal(node),
            NodeKind::Slice(_) => self.infer_slice(node),
            NodeKind::Unary(_) => self.infer_unary(node),
            NodeKind::Binary(binary) => {
                let left = self.infer_value(binary.left);
                let right = self.infer_value(binary.right);

                self.binary_type(binary.operator, left, right)
            }
            NodeKind::Bitwise(bitwise) => {
                let left = self.infer_value(bitwise.left);
                let right = self.infer_value(bitwise.right);

                Self::propagate_unknown(&[left, right], TypeId::NUMBER)
            }
            NodeKind::Logical(logical) => {
                self.infer_value(logical.left);
                self.infer_value(logical.right);

                TypeId::NUMBER
            }
            NodeKind::Isa(isa) => {
                self.infer_value(isa.left);
                self.infer_value(isa.right);

                TypeId::NUMBER
            }
            NodeKind::ComparisonGroup(group) => {
                for &operand in ast.list(group.expressions) {
                    self.infer_value(operand);
                }

                TypeId::NUMBER
            }
            NodeKind::Identifier(_)
            | NodeKind::Member(_)
            | NodeKind::Index(_)
            | NodeKind::Parenthesis(_) => return self.infer_value(node),
            NodeKind::Unknown(_) => TypeId::ANY,
            NodeKind::Import(_)
            | NodeKind::Include(_)
            | NodeKind::ImportCode(_)
            | NodeKind::Envar(_)
            | NodeKind::Inject(_)
            | NodeKind::File(_)
            | NodeKind::Line(_)
            | NodeKind::Debugger(_) => self.infer_directive(node),
            _ => TypeId::UNKNOWN,
        };

        self.record_node_type(node, ty);

        ty
    }

    fn resolve_map(&mut self, node: NodeId, origin: MapOrigin) -> MapId {
        match self.custom_map {
            Some((site, map_id)) if site == self.site(node) => map_id,
            _ => self.model.insert_map_in(origin, self.call_context),
        }
    }

    fn infer_identifier(&mut self, node: NodeId, name: &str) -> TypeId {
        if let Some(slot) = self.resolve_variable(name) {
            self.add_reference(node, &[slot]);
            self.record_global_read(node, slot);

            return self.narrowed_type(slot);
        }

        if let Some(ty) = self.builtin_type(name) {
            return ty;
        }

        if let Some(ty) = self.lookup().general_function_type(name) {
            return ty;
        }

        self.report(
            DiagnosticKind::UnresolvedReference,
            node,
            format!("Cannot find \"{name}\"."),
        );

        TypeId::UNKNOWN
    }

    fn builtin_type(&mut self, name: &str) -> Option<TypeId> {
        let base = match name {
            STRING => Some(BaseType::String),
            NUMBER => Some(BaseType::Number),
            LIST => Some(BaseType::List),
            MAP => Some(BaseType::Map),
            FUNC_REF => Some(BaseType::Function),
            _ => None,
        };

        if let Some(base) = base {
            let map = self.model.insert_map(MapOrigin::Prototype(base));

            return Some(self.object_type(map));
        }

        let dialect = self.api.dialect();

        Some(match name {
            SELF => self.receiver_type(),
            SUPER => {
                let receiver = self.receiver_type();

                self.parents(receiver)
            }
            GLOBALS | LOCALS | OUTER => self.scope_map_type(),
            PARAMS if dialect == Dialect::GreyScript => self.types.list(TypeId::STRING),
            MODULE if dialect != Dialect::MiniScript => {
                let map = self
                    .model
                    .insert_map(MapOrigin::Module(self.current_document));

                self.object_type(map)
            }
            _ => return None,
        })
    }

    fn receiver_type(&self) -> TypeId {
        match self.current_scope {
            ScopeId::Function(function) => self
                .model
                .slot_type(self.model.function(function).receiver_slot),
            ScopeId::Global => TypeId::UNKNOWN,
        }
    }

    fn scope_map_type(&mut self) -> TypeId {
        self.types.map(TypeId::STRING, TypeId::ANY)
    }

    pub(super) fn scope_for_name(&self, node: NodeId) -> Option<ScopeId> {
        let name = self.identifier_name(node)?;

        if !SCOPES.contains(&name) || self.resolve_variable(name).is_some() {
            return None;
        }

        Some(match name {
            GLOBALS => ScopeId::Global,
            LOCALS => self.current_scope,
            _ => self.model.outer_scope(self.current_scope),
        })
    }

    pub(crate) fn literal_key(&self, node: NodeId) -> Option<Key> {
        match self.ast().kind(node) {
            NodeKind::Literal(literal) => match &literal.value {
                LiteralValue::String(text) => {
                    Some(Key::String(self.ast().read(*text).as_ref().into()))
                }
                LiteralValue::Number(number) => {
                    Some(Key::number(if literal.negated { -number } else { *number }))
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn index_type(&mut self, base: TypeId, key: Option<KeyRef<'_>>, key_type: TypeId) -> TypeId {
        let base = self.types.erase_type_parameters(base);

        self.map_variants(base, |this, variant| {
            this.index_variant_type(variant, key, key_type)
        })
    }

    fn index_variant_type(
        &mut self,
        variant: TypeId,
        key: Option<KeyRef<'_>>,
        key_type: TypeId,
    ) -> TypeId {
        match *self.types.get(variant) {
            Type::List(item) => item,
            Type::String => TypeId::STRING,
            Type::Map { value, .. } => value,
            Type::Any => TypeId::ANY,
            Type::Object(map) => match key {
                Some(key) => self.lookup().member_type(variant, key),
                None => self.entry_values_for_key(map, key_type),
            },
            Type::Class(_) => match key {
                Some(key) => self.lookup().member_type(variant, key),
                None => TypeId::ANY,
            },
            _ => TypeId::UNKNOWN,
        }
    }

    fn propagate_unknown(operands: &[TypeId], result: TypeId) -> TypeId {
        if operands.contains(&TypeId::UNKNOWN) {
            TypeId::UNKNOWN
        } else {
            result
        }
    }

    fn binary_type(&mut self, operator: BinaryOp, left: TypeId, right: TypeId) -> TypeId {
        if left == TypeId::UNKNOWN || right == TypeId::UNKNOWN {
            return TypeId::UNKNOWN;
        }

        let left = self.types.erase_type_parameters(left);
        let right = self.types.erase_type_parameters(right);

        if !self.types.is_union(left) && !self.types.is_union(right) {
            return self.binary_variant_type(operator, left, right);
        }

        let pairs: Vec<(TypeId, TypeId)> = self
            .types
            .variants(left)
            .flat_map(|left| self.types.variants(right).map(move |right| (left, right)))
            .collect();
        let results: Vec<TypeId> = pairs
            .into_iter()
            .map(|(left, right)| self.binary_variant_type(operator, left, right))
            .collect();

        self.types.union_all(results)
    }

    fn binary_variant_type(&mut self, operator: BinaryOp, left: TypeId, right: TypeId) -> TypeId {
        let (left_type, right_type) = (self.types.get(left).clone(), self.types.get(right).clone());

        match operator {
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::Greater
            | BinaryOp::LessEqual
            | BinaryOp::GreaterEqual
            | BinaryOp::Modulo
            | BinaryOp::Power => TypeId::NUMBER,
            _ if matches!(left_type, Type::Any) || matches!(right_type, Type::Any) => TypeId::ANY,
            BinaryOp::Add => match (left_type, right_type) {
                (Type::String, _) | (_, Type::String) => TypeId::STRING,
                (Type::List(left_item), Type::List(right_item)) => {
                    let item = self.types.union(left_item, right_item);

                    self.types.list(item)
                }
                (Type::List(_) | Type::Object(_) | Type::Map { .. } | Type::Class(_), _) => left,
                _ => TypeId::NUMBER,
            },
            BinaryOp::Subtract => match left_type {
                Type::String => TypeId::STRING,
                _ => TypeId::NUMBER,
            },
            BinaryOp::Multiply | BinaryOp::Divide => match left_type {
                Type::String | Type::List(_) => left,
                _ => TypeId::NUMBER,
            },
        }
    }
}
