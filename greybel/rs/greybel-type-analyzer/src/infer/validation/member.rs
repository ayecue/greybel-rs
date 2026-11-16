use greybel_core::NodeId;
use rustc_hash::FxHashSet;

use crate::diagnostic::DiagnosticKind;
use crate::infer::Inference;
use crate::infer::call::CallSite;
use crate::model::{KeyRef, MapId, MapOrigin};
use crate::types::{Type, TypeId};

impl Inference<'_> {
    pub(crate) fn validate_member(&mut self, node: NodeId, base: TypeId, name: &str) {
        if self.is_missing_member(base, KeyRef::String(name)) {
            self.report(
                DiagnosticKind::UnknownMember,
                node,
                format!("Cannot find \"{name}\" in this map."),
            );
        }
    }

    fn is_missing_member(&mut self, ty: TypeId, key: KeyRef<'_>) -> bool {
        if key.is_isa() {
            return false;
        }

        let missing_in_all = self.types.variants(ty).all(|variant| {
            let Type::Object(map) = *self.types.get(variant) else {
                return false;
            };

            self.is_missing_from_chain(map, key)
        });

        missing_in_all && ty != TypeId::NEVER && !self.lookup().has_map_member(key)
    }

    fn is_missing_from_chain(&self, map: MapId, key: KeyRef<'_>) -> bool {
        let data = self.model.map(map);

        if !self.is_closed(map) || data.own_entry(key).is_some() {
            return false;
        }

        if self.model.known_slot_type(data.isa).is_none() {
            return true;
        }

        self.model
            .isa_chain(&self.types, map)
            .into_iter()
            .all(|map| self.is_closed(map) && self.model.map(map).own_entry(key).is_none())
    }

    fn is_closed(&self, map: MapId) -> bool {
        let data = self.model.map(map);

        matches!(data.origin, MapOrigin::Literal(_) | MapOrigin::New(_))
            && self.model.known_slot_type(data.dynamic_value).is_none()
            && !self.opaque_call_maps.contains(&map)
            && self.model.known_slot_type(data.isa).is_none_or(|isa| {
                self.types
                    .variants(isa)
                    .all(|parent| matches!(self.types.get(parent), Type::Object(_)))
            })
    }

    pub(crate) fn add_opaque_call_maps(&mut self, site: &CallSite<'_>, with_receiver: bool) {
        let mut types: Vec<TypeId> = site.arguments.to_vec();

        if with_receiver {
            types.extend(site.receiver.map(|receiver| receiver.ty));
        }

        let mut checked_types = FxHashSet::default();

        while let Some(ty) = types.pop() {
            if !checked_types.insert(ty) {
                continue;
            }

            match *self.types.get(ty) {
                Type::Object(map) => {
                    if self.opaque_call_maps.insert(map) {
                        self.pass.has_changes = true;
                    }
                }
                Type::List(item) => types.push(item),
                Type::Map { key, value } => types.extend([key, value]),
                Type::Union(_) => types.extend(self.types.variants(ty)),
                _ => {}
            }
        }
    }
}
