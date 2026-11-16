use greybel_core::{NodeId, NodeKind, UnaryOp};
use meta_utils::BaseType;

use super::Inference;
use super::reference::LiteralKey;
use crate::model::{KeyRef, MapId, MapOrigin, Site, SlotId};
use crate::types::{FunctionRef, Type, TypeId};

impl Inference<'_> {
    pub(crate) fn assign(&mut self, target: NodeId, value: TypeId, init: NodeId) {
        let ast = self.ast();

        match ast.kind(target) {
            NodeKind::Identifier(identifier) => {
                let name = ast.resolve(identifier.name);
                let slot = self
                    .model
                    .insert_variable_by_id(self.current_scope, self.atom_name(identifier.name));

                self.validate_native_override(target, name);
                self.add_assignment(self.current_scope, slot);
                self.assign_variable(target, slot, value);
            }
            NodeKind::Member(member) => {
                let name = self.identifier_name(member.identifier).unwrap_or_default();

                if let Some(scope) = self.scope_for_name(member.base) {
                    let slot = match self.identifier_name_id(member.identifier) {
                        Some(name) => self.model.insert_variable_by_id(scope, name),
                        None => self.model.insert_variable(scope, name),
                    };

                    self.validate_native_override(member.identifier, name);
                    self.add_assignment(scope, slot);
                    self.assign_variable(member.identifier, slot, value);

                    let ty = self.model.slot_type(slot);

                    self.register_node_type(target, ty);

                    return;
                }

                let base = self.infer_value(member.base);
                let key = KeyRef::String(name);

                self.validate_private_member(member.identifier, base, name);
                self.validate_readonly_member(member.identifier, base, name);
                self.validate_constant_entry(target);
                self.assign_entry(base, key, value);
                self.add_instance_entry(base, key, value);

                let name = self.identifier_name_id(member.identifier);

                self.assign_member(target, name, value);

                let slots = self.lookup().entry_slots(base, key);

                self.add_definition(member.identifier, &slots);
                self.bind_receiver(init, base);

                let ty = self.lookup().member_type(base, key);

                self.register_node_type(member.identifier, ty);
                self.register_node_type(target, ty);
            }
            NodeKind::Index(index) => {
                match self.scope_for_name(index.base) {
                    Some(scope) if scope == self.current_scope => self.flow.clear_all(),
                    Some(_) => self.add_unknown_assignment(),
                    None => {}
                }

                let base = self.infer_value(index.base);
                let key = match self.literal_key(index.index) {
                    Some(key) => {
                        self.infer_value(index.index);
                        self.assign_entry(base, key.as_key_ref(), value);
                        self.add_instance_entry(base, key.as_key_ref(), value);

                        if let LiteralKey::String(text) = &key {
                            let name = self.model.insert_name(text);

                            self.assign_member(target, Some(name), value);
                        }

                        let slots = self.lookup().entry_slots(base, key.as_key_ref());

                        self.add_definition(index.index, &slots);
                        self.bind_receiver(init, base);

                        match key {
                            LiteralKey::String(_) => TypeId::STRING,
                            LiteralKey::Number(_) => TypeId::NUMBER,
                        }
                    }
                    None => {
                        let key = self.infer_value(index.index);

                        if self.has_members(base) {
                            self.assign_member(target, None, value);
                        }

                        self.assign_dynamic_entry(base, key, value);

                        key
                    }
                };

                self.assign_into_containers(index.base, base, key, value);
                self.register_node_type(target, value);
            }
            _ => {
                self.infer_value(target);
            }
        }
    }

    pub(super) fn assign_variable(&mut self, node: NodeId, slot: SlotId, value: TypeId) {
        self.widen(slot, value);

        if value == TypeId::UNKNOWN {
            self.flow.clear(slot);
        } else {
            self.flow.assign(slot, value);
        }

        self.add_definition(node, &[slot]);

        let ty = if value == TypeId::UNKNOWN {
            self.model.slot_type(slot)
        } else {
            value
        };

        self.register_node_type(node, ty);
    }

    fn has_members(&self, ty: TypeId) -> bool {
        self.types.variants(ty).any(|variant| {
            matches!(
                self.types.get(variant),
                Type::Map { .. } | Type::Object(_) | Type::Class(_) | Type::Any | Type::Unknown
            )
        })
    }

    pub(super) fn bind_receiver(&mut self, init: NodeId, receiver: TypeId) {
        if let NodeKind::Function(_) = self.ast().kind(init) {
            let function = self.function_id(init);
            let slot = self.model.function(function).receiver_slot;
            let receiver = self.map_variants(receiver, Self::prototype_receiver);
            let receiver = self.receiver_with_instances(receiver);

            self.widen(slot, receiver);
        }
    }

    fn prototype_receiver(&mut self, variant: TypeId) -> TypeId {
        let Some(map) = self.types.object_map(variant) else {
            return variant;
        };
        let MapOrigin::Prototype(base) = self.model.map(map).origin else {
            return variant;
        };

        match base {
            BaseType::List => self.types.list(TypeId::ANY),
            BaseType::Map => self.types.map(TypeId::ANY, TypeId::ANY),
            BaseType::String => TypeId::STRING,
            BaseType::Number => TypeId::NUMBER,
            BaseType::Function => self.types.function(FunctionRef::Any),
            _ => variant,
        }
    }

    pub(super) fn custom_map_at(&mut self, node: NodeId, init: NodeId) -> Option<(Site, MapId)> {
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
}
