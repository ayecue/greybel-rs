use super::Inference;
use crate::model::{FunctionId, KeyRef, MapId, MapOrigin, ScopeId};
use crate::types::TypeId;

impl Inference<'_> {
    pub(super) fn add_instance(&mut self, instance: MapId) {
        for class in self.model.isa_chain(&self.types, instance) {
            if class != instance && self.classes.insert(class) {
                self.pass.has_changes = true;
            }
        }
    }

    fn instance_map(&mut self, class: MapId) -> MapId {
        let map = self.model.insert_map(MapOrigin::Instance(class));
        let parents: Vec<MapId> = self
            .types
            .object_maps(self.model.slot_type(self.model.map(class).isa))
            .filter(|parent| self.classes.contains(parent))
            .collect();
        let mut isa = vec![self.object_type(class)];

        for parent in parents {
            let parent_instances = self.model.insert_map(MapOrigin::Instance(parent));

            isa.push(self.object_type(parent_instances));
        }

        let isa = self.types.union_all(isa);

        self.widen(self.model.map(map).isa, isa);

        map
    }

    pub(super) fn receiver_with_instances(&mut self, receiver: TypeId) -> TypeId {
        let classes: Vec<MapId> = self
            .types
            .object_maps(receiver)
            .filter(|class| self.classes.contains(class))
            .collect();

        if classes.is_empty() {
            return receiver;
        }

        let mut variants = vec![receiver];

        for class in classes {
            let instances = self.instance_map(class);

            variants.push(self.object_type(instances));
        }

        self.types.union_all(variants)
    }

    pub(super) fn without_instance_maps(&mut self, ty: TypeId) -> TypeId {
        if !self.types.is_union(ty) {
            return ty;
        }

        let variants: Vec<TypeId> = self
            .types
            .variants(ty)
            .filter(|&variant| {
                self.types
                    .object_map(variant)
                    .is_none_or(|map| !matches!(self.model.map(map).origin, MapOrigin::Instance(_)))
            })
            .collect();

        self.types.union_all(variants)
    }

    pub(super) fn add_instance_entry(&mut self, base: TypeId, key: KeyRef<'_>, value: TypeId) {
        let ScopeId::Function(function) = self.current_scope else {
            return;
        };
        let classes = self.method_classes(function);

        if classes.is_empty() {
            return;
        }

        let instances: Vec<MapId> = self
            .types
            .object_maps(base)
            .filter(|&map| {
                !matches!(self.model.map(map).origin, MapOrigin::Instance(_))
                    && !classes.contains(&map)
            })
            .collect();

        for instance in instances {
            let chain = self.model.isa_chain(&self.types, instance);

            for &class in classes.iter().filter(|class| chain.contains(class)) {
                let instances = self.instance_map(class);
                let instance_type = self.object_type(instances);

                self.assign_entry(instance_type, key, value);
            }
        }
    }

    fn method_classes(&self, function: FunctionId) -> Vec<MapId> {
        let data = self.model.function(function);
        let function = data.context.map_or(function, |context| {
            self.model.call_context(context).function
        });
        let receiver = self
            .model
            .slot_type(self.model.function(function).receiver_slot);

        self.types
            .object_maps(receiver)
            .filter(|class| self.classes.contains(class))
            .collect()
    }
}
