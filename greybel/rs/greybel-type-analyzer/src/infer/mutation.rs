use greybel_core::{NodeId, NodeKind};
use greybel_symbols::names::SELF;

use super::Inference;
use super::call::{CallSite, Receiver};
use super::generic::{BindingTable, Evidence};
use crate::model::{GenericInstance, MapId, MapOrigin, ScopeId, SlotId};
use crate::types::{FunctionRef, Type, TypeId};

pub(super) struct Mutation {
    target: Target,
    pattern: TypeId,
    fixed_bindings: Vec<(Box<str>, TypeId)>,
}

enum Target {
    Receiver(Receiver),
    Argument { node: NodeId, ty: TypeId },
}

impl Mutation {
    pub(super) fn targets_receiver(&self) -> bool {
        matches!(self.target, Target::Receiver(_))
    }
}

impl Inference<'_> {
    pub(super) fn call_mutation(
        &mut self,
        function: FunctionRef,
        site: &CallSite<'_>,
    ) -> Option<Mutation> {
        match function {
            FunctionRef::Any => None,
            FunctionRef::ApiMethod(id) => {
                self.call_mutation(FunctionRef::Api(self.model.api_method(id).function), site)
            }
            FunctionRef::Api(api) => {
                let fixed_bindings = self.lookup().api_mutation(api)?;
                let pattern = self.lookup().api_receiver_type(api)?;

                Some(Mutation {
                    target: Target::Receiver(site.receiver?),
                    pattern,
                    fixed_bindings,
                })
            }
            FunctionRef::Body(id) => {
                let data = self.model.function(id);
                let target = data.doc.as_ref()?.mutated_parameter.clone()?;
                let parameters: Vec<(String, SlotId)> = data
                    .parameters(site.method)
                    .iter()
                    .map(|parameter| (parameter.name.clone(), parameter.slot))
                    .collect();
                let index = parameters.iter().position(|(name, _)| *name == target);
                let pattern = index.map(|index| self.model.slot_type(parameters[index].1));

                self.target_mutation(&target, index, pattern, site)
            }
            FunctionRef::Signature(id) => {
                let original = self
                    .model
                    .declared_function(id)
                    .generic_source
                    .unwrap_or(id);
                let declared_function = self.model.declared_function(original);
                let target = declared_function.mutated_parameter.clone()?;
                let index = declared_function
                    .parameters
                    .iter()
                    .position(|parameter| parameter.name == target);
                let pattern = index.map(|index| declared_function.parameters[index].ty);

                self.target_mutation(&target, index, pattern, site)
            }
        }
    }

    fn target_mutation(
        &mut self,
        target: &str,
        index: Option<usize>,
        pattern: Option<TypeId>,
        site: &CallSite<'_>,
    ) -> Option<Mutation> {
        let (target, pattern) = if target == SELF && index.is_none() {
            let receiver = site.receiver?;
            let pattern = self.generic_pattern(receiver.ty)?;

            (Target::Receiver(receiver), pattern)
        } else {
            let index = index?;
            let target = Target::Argument {
                node: *site.argument_nodes.get(index)?,
                ty: *site.arguments.get(index)?,
            };

            (target, pattern?)
        };

        Some(Mutation {
            target,
            pattern,
            fixed_bindings: Vec::new(),
        })
    }

    fn generic_pattern(&mut self, ty: TypeId) -> Option<TypeId> {
        let generic = self.types.variants(ty).find_map(|variant| {
            if let Some(specialization) = self.model.generic_instance_of(&self.types, variant) {
                return Some(specialization.generic);
            }

            let map = self.types.object_map(variant)?;
            let custom = self.model.custom_type_of(map)?;

            (!custom.type_parameters.is_empty()).then_some(map)
        })?;
        let names = self.model.custom_type_of(generic)?.type_parameters.clone();
        let arguments = names
            .iter()
            .map(|name| self.types.type_parameter(name))
            .collect();

        Some(self.instantiate(generic, arguments))
    }

    pub(super) fn apply_mutation(
        &mut self,
        effect: &Mutation,
        evidence: &Evidence,
    ) -> Option<TypeId> {
        let mut names = Vec::new();

        self.type_parameter_names(effect.pattern, &mut names);

        let bindings: BindingTable = names
            .into_iter()
            .map(|name| {
                let mut types = evidence.get(&name).cloned().unwrap_or_default();

                types.extend(
                    effect
                        .fixed_bindings
                        .iter()
                        .filter(|(fixed_binding, _)| *fixed_binding == name)
                        .map(|&(_, ty)| ty),
                );

                let bound = self.types.union_all(types);

                (name, bound)
            })
            .collect();
        let (node, current) = match effect.target {
            Target::Receiver(receiver) => (receiver.node, receiver.ty),
            Target::Argument { node, ty } => (node, ty),
        };

        if let Type::Map { key, value } = *self.types.get(effect.pattern) {
            let key = self.substitute(key, &bindings);
            let value = self.substitute(value, &bindings);
            let maps: Vec<MapId> = self.types.object_maps(current).collect();

            for map in maps {
                let object = self.object_type(map);

                self.assign_dynamic_entry(object, key, value);
            }
        }

        let container_type = self.substitute(effect.pattern, &bindings);

        if !self.has_shape_of(current, effect.pattern) {
            return None;
        }

        for slot in self.target_slots(node) {
            self.widen(slot, container_type);
        }

        Some(container_type)
    }

    fn has_shape_of(&self, ty: TypeId, pattern: TypeId) -> bool {
        let generic = self
            .model
            .generic_instance_of(&self.types, pattern)
            .map(|specialization| specialization.generic);

        self.types.variants(ty).any(|variant| {
            match (self.types.get(variant), self.types.get(pattern)) {
                (Type::List(_), Type::List(_)) | (Type::Map { .. }, Type::Map { .. }) => true,
                (Type::Object(map), Type::Object(_)) => generic.is_some_and(|generic| {
                    *map == generic
                        || self
                            .model
                            .generic_instance_of(&self.types, variant)
                            .is_some_and(|specialization| specialization.generic == generic)
                }),
                _ => false,
            }
        })
    }

    fn target_slots(&mut self, node: NodeId) -> Vec<SlotId> {
        let ast = self.ast();

        match ast.kind(node) {
            NodeKind::Parenthesis(parenthesis) => self.target_slots(parenthesis.expression),
            NodeKind::Identifier(_) => {
                let slots = self.references.slots(self.site(node)).to_vec();

                match self.current_scope {
                    ScopeId::Function(function)
                        if slots.is_empty() && self.identifier_name(node) == Some(SELF) =>
                    {
                        vec![self.model.function(function).receiver_slot]
                    }
                    _ => slots,
                }
            }
            NodeKind::Member(member) => {
                self.references.slots(self.site(member.identifier)).to_vec()
            }
            NodeKind::Index(index) => {
                let Some(key) = self.literal_key(index.index) else {
                    return Vec::new();
                };
                let base = self.node_type(index.base).unwrap_or(TypeId::UNKNOWN);

                self.lookup().entry_slots(base, key.as_key_ref()).to_vec()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn assign_into_containers(
        &mut self,
        target: NodeId,
        base: TypeId,
        key: TypeId,
        value: TypeId,
    ) {
        let (mut list, mut map) = (false, false);

        for variant in self.types.variants(base) {
            match self.types.get(variant) {
                Type::List(_) => list = true,
                Type::Map { .. } => map = true,
                _ => {}
            }
        }

        let mut container_types = Vec::new();

        if list {
            container_types.push(self.types.list(value));
        }

        if map {
            container_types.push(self.types.map(key, value));
        }

        if container_types.is_empty() {
            return;
        }

        let container_type = self.types.union_all(container_types);

        for slot in self.target_slots(target) {
            self.widen(slot, container_type);
        }
    }

    pub(super) fn merge_generic_instances(&mut self, ty: TypeId) -> TypeId {
        let Type::Union(variants) = self.types.get(ty) else {
            return ty;
        };
        let specialized_variants = variants
            .iter()
            .filter(|&&variant| {
                self.types.object_map(variant).is_some_and(|map| {
                    matches!(self.model.map(map).origin, MapOrigin::GenericInstance(_))
                })
            })
            .count();

        if specialized_variants < 2 {
            return ty;
        }

        let mut groups: Vec<(MapId, Vec<Vec<TypeId>>)> = Vec::new();
        let mut remainder = Vec::new();

        for variant in self.types.variants(ty).collect::<Vec<_>>() {
            let specialization =
                self.types
                    .object_map(variant)
                    .and_then(|map| match self.model.map(map).origin {
                        MapOrigin::GenericInstance(id) => {
                            Some(self.model.generic_instance(id).clone())
                        }
                        _ => None,
                    });

            match specialization {
                Some(GenericInstance { generic, arguments }) => {
                    match groups
                        .iter_mut()
                        .find(|(candidate, _)| *candidate == generic)
                    {
                        Some((_, lists)) => lists.push(arguments),
                        None => groups.push((generic, vec![arguments])),
                    }
                }
                None => remainder.push(variant),
            }
        }

        if groups.iter().all(|(_, lists)| lists.len() < 2) {
            return ty;
        }

        for (generic, lists) in groups {
            let width = lists.iter().map(Vec::len).max().unwrap_or(0);
            let arguments = (0..width)
                .map(|position| {
                    let types: Vec<TypeId> = lists
                        .iter()
                        .filter_map(|arguments| arguments.get(position).copied())
                        .collect();

                    self.types.union_all(types)
                })
                .collect();

            remainder.push(self.instantiate(generic, arguments));
        }

        self.types.union_all(remainder)
    }
}
