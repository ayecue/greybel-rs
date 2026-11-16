use std::borrow::Cow;

use greybel_core::{Dialect, LiteralValue, NodeId, NodeKind};
use greybel_symbols::names::{GLOBALS, LOCALS, MODULE, OUTER, PARAMS, SCOPES, SELF, SUPER};
use greybel_symbols::types::{FUNC_REF, LIST, MAP, NUMBER, STRING};
use meta_utils::BaseType;

use super::Inference;
use super::call::Receiver;
use crate::diagnostic::DiagnosticKind;
use crate::model::{KeyRef, MapOrigin, NameId, ScopeId};
use crate::types::{Type, TypeId};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LiteralKey<'a> {
    String(Cow<'a, str>),
    Number(KeyRef<'static>),
}

impl LiteralKey<'_> {
    pub(crate) fn as_key_ref(&self) -> KeyRef<'_> {
        match self {
            Self::String(text) => KeyRef::String(text),
            Self::Number(key) => *key,
        }
    }
}

impl<'a> Inference<'a> {
    pub(crate) fn infer_reference(&mut self, node: NodeId) -> TypeId {
        self.infer_reference_with_receiver(node).0
    }

    pub(super) fn infer_reference_with_receiver(
        &mut self,
        node: NodeId,
    ) -> (TypeId, Option<Receiver>) {
        let ast = self.ast();
        let mut receiver = None;
        let ty = match ast.kind(node) {
            NodeKind::Identifier(identifier) => self.infer_identifier(
                node,
                self.atom_name(identifier.name),
                ast.resolve(identifier.name),
            ),
            NodeKind::Member(member) => {
                let mut optional_null = false;
                let name = self.identifier_name(member.identifier).unwrap_or_default();
                let ty = match self.scope_for_name(member.base) {
                    Some(scope) => {
                        let scope_map = self.scope_map_type();

                        self.register_node_type(member.base, scope_map);

                        let variable = self
                            .identifier_name_id(member.identifier)
                            .and_then(|name| self.model.variable_by_id(scope, name));

                        match variable {
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

                        let optional =
                            member.optional || self.continues_optional_chain(member.base);

                        optional_null = optional && self.includes_null(base);

                        let base = self.optional_base(optional, base);

                        receiver = Some(Receiver {
                            node: member.base,
                            ty: base,
                        });

                        let key = KeyRef::String(name);
                        let slots = self.lookup().entry_slots(base, key);

                        if slots.is_empty() {
                            self.validate_member(member.identifier, base, name);
                        }

                        self.validate_private_member(member.identifier, base, name);
                        self.validate_optional_member(node, member.identifier, base, name);

                        self.add_reference(member.identifier, &slots);

                        let ty = self.lookup().member_type(base, key);

                        self.member_fact_type(node, ty)
                    }
                };
                let ty = self.optional_result(optional_null, ty);
                let method = match receiver {
                    Some(receiver) => self.method_type(ty, receiver.ty),
                    None => ty,
                };

                self.register_node_type(member.identifier, method);
                self.register_node_type(node, method);

                return (ty, receiver);
            }
            NodeKind::Index(index) => {
                let base = self.infer_value(index.base);
                let optional = index.optional || self.continues_optional_chain(index.base);
                let optional_null = optional && self.includes_null(base);
                let base = self.optional_base(optional, base);

                receiver = Some(Receiver {
                    node: index.base,
                    ty: base,
                });
                self.validate_index(node, base);

                let ty = match self.literal_key(index.index) {
                    Some(key) => {
                        self.infer_value(index.index);

                        let ty = self.index_type(base, Some(key.as_key_ref()), TypeId::UNKNOWN);

                        self.member_fact_type(node, ty)
                    }
                    None => {
                        let key = self.infer_value(index.index);

                        self.index_type(base, None, key)
                    }
                };

                self.optional_result(optional_null, ty)
            }
            NodeKind::Parenthesis(parenthesis) => {
                return self.infer_reference_with_receiver(parenthesis.expression);
            }
            _ => return (self.infer_expression(node), None),
        };

        self.register_node_type(node, ty);

        (ty, receiver)
    }

    pub(super) fn infer_identifier(&mut self, node: NodeId, name_id: NameId, name: &str) -> TypeId {
        if let Some(slot) = self.resolve_variable(name_id) {
            self.add_reference(node, &[slot]);

            return self.narrowed_type(slot);
        }

        if let Some(ty) = self.builtin_type(name) {
            return ty;
        }

        if let Some(ty) = self.lookup().general_function_type(name) {
            return ty;
        }

        let later_variable = self
            .model
            .visible_variables(self.current_scope, name_id)
            .next();

        if let Some(slot) = later_variable {
            self.add_reference(node, &[slot]);
            self.report(
                DiagnosticKind::UseBeforeDefinition,
                node,
                format!("\"{name}\" is used before it is assigned."),
            );

            return self.model.slot_type(slot);
        }

        self.report(
            DiagnosticKind::UnresolvedReference,
            node,
            format!("Cannot find \"{name}\"."),
        );

        TypeId::UNKNOWN
    }

    pub(super) fn builtin_type(&mut self, name: &str) -> Option<TypeId> {
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

    pub(super) fn receiver_type(&self) -> TypeId {
        match self.current_scope {
            ScopeId::Function(function) => self
                .model
                .slot_type(self.model.function(function).receiver_slot),
            ScopeId::Global => TypeId::UNKNOWN,
        }
    }

    pub(super) fn scope_map_type(&mut self) -> TypeId {
        self.types.map(TypeId::STRING, TypeId::ANY)
    }

    pub(super) fn scope_for_name(&self, node: NodeId) -> Option<ScopeId> {
        let name = self.identifier_name(node)?;

        if !SCOPES.contains(&name)
            || self
                .resolve_variable(self.identifier_name_id(node)?)
                .is_some()
        {
            return None;
        }

        Some(match name {
            GLOBALS => ScopeId::Global,
            LOCALS => self.current_scope,
            _ => self.model.outer_scope(self.current_scope),
        })
    }

    pub(crate) fn literal_key(&self, node: NodeId) -> Option<LiteralKey<'a>> {
        let ast = self.ast();

        match ast.kind(node) {
            NodeKind::Literal(literal) => match &literal.value {
                LiteralValue::String(text) => Some(LiteralKey::String(ast.read(*text))),
                LiteralValue::Number(number) => {
                    let number = if literal.negated { -number } else { *number };

                    Some(LiteralKey::Number(KeyRef::number(number)))
                }
                _ => None,
            },
            _ => None,
        }
    }

    pub(super) fn index_type(
        &mut self,
        base: TypeId,
        key: Option<KeyRef<'_>>,
        key_type: TypeId,
    ) -> TypeId {
        let base = self.types.erase_type_parameters(base);

        self.map_variants(base, |this, variant| {
            this.index_variant_type(variant, key, key_type)
        })
    }

    pub(super) fn index_variant_type(
        &mut self,
        variant: TypeId,
        key: Option<KeyRef<'_>>,
        key_type: TypeId,
    ) -> TypeId {
        match *self.types.get(self.types.without_literal(variant)) {
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

    /// Whether `base` is a chain with a `?.` link, so a `null` base ends the chain and the links after it don't
    /// run: `m?.n.k` is `null` when `m` is.
    pub(super) fn continues_optional_chain(&self, base: NodeId) -> bool {
        let ast = self.ast();
        let mut current = base;

        loop {
            let (link_base, optional) = match ast.kind(current) {
                NodeKind::Member(member) => (member.base, member.optional),
                NodeKind::Index(index) => (index.base, index.optional),
                NodeKind::Slice(slice) => (slice.base, slice.optional),
                NodeKind::Call(call) => (call.base, call.optional),
                _ => return false,
            };

            if optional {
                return true;
            }

            current = link_base;
        }
    }

    /// The base of a `?.` link without `null`, which ends the chain instead.
    fn optional_base(&mut self, optional: bool, base: TypeId) -> TypeId {
        if optional {
            self.without_null(base)
        } else {
            base
        }
    }

    /// The value of a `?.` link: `null` too when its base could be `null`.
    fn optional_result(&mut self, optional_null: bool, ty: TypeId) -> TypeId {
        if optional_null {
            return self.types.union(ty, TypeId::NULL);
        }

        ty
    }
}
