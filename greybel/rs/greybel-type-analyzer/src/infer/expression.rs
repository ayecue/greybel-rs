use std::mem;

use greybel_core::syntax::greybel::{AssertedType, TypeAssertion};
use greybel_core::{BinaryOp, LiteralValue, LogicalOp, NodeId, NodeKind, UnaryOp};

use super::Inference;
use super::call::CallSite;
use super::flow::KindSet;
use super::validation::{Evaluation, Operand};
use crate::annotation::TypeExpr;
use crate::diagnostic::DiagnosticKind;
use crate::model::{Key, MapId, MapOrigin};
use crate::types::{FunctionRef, Type, TypeId};

impl<'a> Inference<'a> {
    pub(crate) fn infer_value(&mut self, node: NodeId) -> TypeId {
        let ty = self.infer_value_type(node);

        if ty == TypeId::NEVER {
            self.flow.set_unreachable();
        }

        ty
    }

    fn infer_value_type(&mut self, node: NodeId) -> TypeId {
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

                self.register_node_type(node, ty);

                ty
            }
            _ => self.infer_expression(node),
        }
    }

    fn infer_ternary(&mut self, node: NodeId) -> TypeId {
        let NodeKind::Ternary(ternary) = self.ast().kind(node) else {
            unreachable!()
        };

        self.infer_value(ternary.condition);

        let guard = self.condition_guard(ternary.condition);
        let entry = self.flow.clone();

        self.refine_flow(&guard.when_true);

        let if_true = self.infer_value(ternary.if_true);
        let true_state = mem::replace(&mut self.flow, entry);

        self.refine_flow(&guard.when_false);

        let if_false = self.infer_value(ternary.if_false);
        let false_state = mem::take(&mut self.flow);

        self.flow = self.join_flows(true_state, &false_state);

        self.types.union(if_true, if_false)
    }

    fn infer_coalesce(&mut self, node: NodeId) -> TypeId {
        let NodeKind::Coalesce(coalesce) = self.ast().kind(node) else {
            unreachable!()
        };
        let left = self.infer_value(coalesce.left);

        if !self.includes_null(left) {
            return left;
        }

        let skip_state = self.flow.clone();
        let right = self.infer_value(coalesce.right);
        let right_state = mem::take(&mut self.flow);

        self.flow = self.join_flows(skip_state, &right_state);

        let present = self.without_null(left);

        self.types.union(present, right)
    }

    pub(super) fn includes_null(&self, ty: TypeId) -> bool {
        self.types.variants(ty).any(|variant| {
            matches!(
                self.types.get(variant),
                Type::Null | Type::Any | Type::Unknown
            )
        })
    }

    pub(super) fn without_null(&mut self, ty: TypeId) -> TypeId {
        self.restrict_to_kinds(ty, !KindSet::NULL)
    }

    fn register_guard_call(&mut self, node: NodeId, callee: TypeId, method: bool) {
        let Type::Function(FunctionRef::Body(function)) = *self.types.get(callee) else {
            return;
        };
        let data = self.model.function(function);
        let Some(TypeExpr::Guard { parameter, ty }) =
            data.doc.as_ref().and_then(|doc| doc.returns.clone())
        else {
            return;
        };
        let Some(index) = data
            .parameters(method)
            .iter()
            .position(|candidate| candidate.name == parameter)
        else {
            return;
        };
        let ty = self.resolve_type(&ty);

        self.guard_calls
            .insert((self.current_document, node), (index, ty));
    }

    fn infer_call(&mut self, node: NodeId) -> TypeId {
        let ast = self.ast();
        let NodeKind::Call(call) = ast.kind(node) else {
            unreachable!()
        };
        let (callee, receiver) = self.infer_reference_with_receiver(call.base);
        let optional = call.optional || self.continues_optional_chain(call.base);
        let optional_null = optional && self.includes_null(callee);
        let callee = if optional {
            self.without_null(callee)
        } else {
            callee
        };
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

        self.register_guard_call(node, callee, method);
        self.validate_call(node, callee, &arguments, call.arguments, method);

        let site = CallSite {
            method,
            receiver,
            arguments: &arguments,
            argument_nodes: ast.list(call.arguments),
        };
        let result = self.call_result_type(callee, &site);
        let (mut opaque_callee, mut unknown_callee) = (false, false);

        for variant in self.types.variants(callee) {
            match self.types.get(variant) {
                Type::Any => (opaque_callee, unknown_callee) = (true, true),
                Type::Unknown => unknown_callee = true,
                Type::Function(FunctionRef::Any) => opaque_callee = true,
                _ => {}
            }
        }

        if unknown_callee {
            self.add_unknown_assignment();
        }

        if opaque_callee || unknown_callee {
            self.add_opaque_call_maps(&site, opaque_callee);
        }

        self.type_buffers.release(arguments);

        if optional_null {
            return self.types.union(result, TypeId::NULL);
        }

        result
    }

    fn infer_list(&mut self, node: NodeId) -> TypeId {
        let ast = self.ast();
        let NodeKind::List(list) = ast.kind(node) else {
            unreachable!()
        };
        let mut items = self.type_buffers.take();

        for &item in ast.list(list.fields) {
            let ty = match ast.kind(item) {
                NodeKind::Spread(spread) => {
                    let argument = self.infer_value(spread.argument);
                    let elements = self.spread_elements(argument);

                    self.register_node_type(item, argument);

                    elements
                }
                _ => self.infer_value(item),
            };

            items.push(ty);
        }

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
            if let NodeKind::Spread(spread) = ast.kind(entry) {
                let argument = self.infer_value(spread.argument);

                self.spread_entries(object, argument);
                self.register_node_type(entry, argument);

                continue;
            }

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
                            format!("Duplicate key {text} in the map literal."),
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

            self.register_node_type(entry, value);
        }

        object
    }

    fn spread_elements(&mut self, argument: TypeId) -> TypeId {
        let elements: Vec<TypeId> = self
            .types
            .variants(argument)
            .filter_map(|variant| match *self.types.get(variant) {
                Type::List(item) => Some(item),
                Type::Any | Type::Unknown => Some(TypeId::ANY),
                _ => None,
            })
            .collect();

        self.types.union_all(elements)
    }

    fn spread_entries(&mut self, object: TypeId, argument: TypeId) {
        let maps: Vec<MapId> = self.types.object_maps(argument).collect();
        let opaque = maps.is_empty()
            || self
                .types
                .variants(argument)
                .any(|variant| !matches!(self.types.get(variant), Type::Object(_)));

        for map in maps {
            let entries: Vec<(Key, TypeId)> = self
                .model
                .map(map)
                .entries
                .iter()
                .map(|(key, &slot)| (key.clone(), self.model.slot_type(slot)))
                .collect();

            for (key, ty) in entries {
                self.assign_entry(object, key.as_key_ref(), ty);
            }
        }

        if opaque {
            self.assign_dynamic_entry(object, TypeId::ANY, TypeId::ANY);
        }
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
            .map(|variant| self.types.without_literal(variant))
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
                self.add_instance(map);

                self.object_type(map)
            }
            UnaryOp::Not => {
                let operand = self.infer_value(unary.argument);

                Self::propagate_unknown(&[operand], TypeId::NUMBER)
            }
            UnaryOp::Negate => {
                let operand = self.infer_value(unary.argument);

                self.validate_negation(node, (unary.argument, operand));

                let results: Vec<TypeId> = self
                    .types
                    .variants(operand)
                    .map(|variant| match Operand::of(self.types.get(variant)) {
                        Some(operand) if !operand.is_number_or_null() => TypeId::NULL,
                        _ => TypeId::NUMBER,
                    })
                    .collect();
                let result = self.types.union_all(results);

                Self::propagate_unknown(&[operand], result)
            }
        }
    }

    fn infer_function_value(&mut self, node: NodeId) -> TypeId {
        self.validate_written_types(node);

        let function = self.infer_function_body(node);

        self.types.function(FunctionRef::Body(function))
    }

    fn infer_type_assertion(&mut self, assertion: &TypeAssertion) -> TypeId {
        let ty = self.infer_value(assertion.expression);

        match assertion.target {
            AssertedType::NonNull => self.without_null(ty),
            AssertedType::Type(span) => self
                .written_type_expr(span)
                .map(|expr| self.resolve_type(&expr))
                .filter(|&ty| ty != TypeId::UNKNOWN)
                .unwrap_or(ty),
        }
    }

    pub(super) fn infer_expression(&mut self, node: NodeId) -> TypeId {
        let ast = self.ast();
        let ty = match ast.kind(node) {
            NodeKind::Literal(literal) => match literal.value {
                LiteralValue::String(_) => TypeId::STRING,
                LiteralValue::Number(_) | LiteralValue::Boolean(_) => TypeId::NUMBER,
                LiteralValue::Nil => TypeId::NULL,
            },
            NodeKind::Function(_) => self.infer_function_value(node),
            NodeKind::Call(_) => self.infer_call(node),
            NodeKind::List(_) => self.infer_list(node),
            NodeKind::ListItem(item) => self.infer_value(item.value),
            NodeKind::Map(_) => self.infer_map_literal(node),
            NodeKind::Slice(_) => self.infer_slice(node),
            NodeKind::Unary(_) => self.infer_unary(node),
            NodeKind::Binary(binary) => {
                let left = self.infer_value(binary.left);
                let right = self.infer_value(binary.right);

                self.validate_binary(
                    node,
                    binary.operator,
                    (binary.left, left),
                    (binary.right, right),
                );

                self.binary_type(binary.operator, left, right)
            }
            NodeKind::Bitwise(bitwise) => {
                let left = self.infer_value(bitwise.left);
                let right = self.infer_value(bitwise.right);

                self.validate_bitwise(
                    node,
                    bitwise.operator.punct().as_str(),
                    (bitwise.left, left),
                    (bitwise.right, right),
                );

                let result = self.numeric_result(left, right);

                Self::propagate_unknown(&[left, right], result)
            }
            NodeKind::Logical(logical) => {
                self.infer_value(logical.left);

                let skip_state = self.flow.clone();
                let guard = self.condition_guard(logical.left);

                self.refine_flow(match logical.operator {
                    LogicalOp::And => &guard.when_true,
                    LogicalOp::Or => &guard.when_false,
                });
                self.infer_value(logical.right);

                let right_state = mem::take(&mut self.flow);

                self.flow = self.join_flows(skip_state, &right_state);

                TypeId::NUMBER
            }
            NodeKind::Isa(isa) => {
                self.infer_value(isa.left);
                self.infer_value(isa.right);

                TypeId::NUMBER
            }
            NodeKind::Ternary(_) => self.infer_ternary(node),
            NodeKind::Coalesce(_) => self.infer_coalesce(node),
            NodeKind::TypeAssertion(assertion) => self.infer_type_assertion(assertion),
            NodeKind::ComparisonGroup(group) => {
                for &operand in ast.list(group.expressions) {
                    self.infer_value(operand);
                }

                TypeId::NUMBER
            }
            NodeKind::Template(template) => {
                for &expression in ast.list(template.expressions) {
                    self.infer_value(expression);
                }

                TypeId::STRING
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

        self.register_node_type(node, ty);

        ty
    }

    fn resolve_map(&mut self, node: NodeId, origin: MapOrigin) -> MapId {
        match self.custom_map {
            Some((site, map_id)) if site == self.site(node) => map_id,
            _ => self.model.insert_map_in(origin, self.context.current),
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

    fn numeric_result(&mut self, left: TypeId, right: TypeId) -> TypeId {
        let mut results = Vec::new();

        for left in self.types.variants(left) {
            for right in self.types.variants(right) {
                let operands = (
                    Operand::of(self.types.get(left)),
                    Operand::of(self.types.get(right)),
                );

                results.push(match operands {
                    (Some(left), Some(right))
                        if left != Operand::Number || !right.is_number_or_null() =>
                    {
                        TypeId::NULL
                    }
                    _ => TypeId::NUMBER,
                });
            }
        }

        self.types.union_all(results)
    }

    fn binary_variant_type(&mut self, operator: BinaryOp, left: TypeId, right: TypeId) -> TypeId {
        let (left, right) = (
            self.types.without_literal(left),
            self.types.without_literal(right),
        );
        let (left_type, right_type) = (self.types.get(left).clone(), self.types.get(right).clone());

        if let (Some(left), Some(right)) = (Operand::of(&left_type), Operand::of(&right_type))
            && Evaluation::of_binary(operator, left, right) == Evaluation::Null
        {
            return TypeId::NULL;
        }

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
