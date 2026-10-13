use greybel_symbols::names::SELF;

use super::Inference;
use super::generic::{BindingTable, Evidence};
use crate::model::{ApiMethod, Key, KeyRef, MapId};
use crate::types::{ApiFunction, FunctionRef, Type, TypeId};

impl Inference<'_> {
    pub(crate) fn method_type(&mut self, ty: TypeId, receiver: TypeId) -> TypeId {
        self.map_variants(ty, |this, variant| match *this.types.get(variant) {
            Type::Function(FunctionRef::Api(function)) => {
                this.api_method(function, receiver).unwrap_or(variant)
            }
            _ => variant,
        })
    }

    fn api_method(&mut self, function: ApiFunction, receiver: TypeId) -> Option<TypeId> {
        let parameters = self.lookup().api_parameter_types(function);
        let returns = self.lookup().return_type(FunctionRef::Api(function));
        let mut names = Vec::new();

        for &ty in parameters.iter().chain([&returns]) {
            self.type_parameter_names(ty, &mut names);
        }

        if names.is_empty() {
            return None;
        }

        let mut evidence = Evidence::default();

        if let Some(pattern) = self.lookup().api_receiver_type(function) {
            self.bind(pattern, receiver, &mut evidence);
        }

        if receiver != TypeId::UNKNOWN {
            evidence.entry(SELF.into()).or_default().push(receiver);
        }

        let bindings: BindingTable = names
            .into_iter()
            .filter_map(|name| {
                let types = evidence.get(&name)?.clone();

                Some((name, self.types.union_all(types)))
            })
            .collect();
        let parameters = parameters
            .into_iter()
            .map(|ty| self.substitute(ty, &bindings))
            .collect();
        let returns = self.substitute(returns, &bindings);
        let method = self.model.insert_api_method(ApiMethod {
            function,
            parameters,
            returns,
        });

        Some(self.types.function(FunctionRef::ApiMethod(method)))
    }

    pub(crate) fn parents(&mut self, ty: TypeId) -> TypeId {
        let parents: Vec<TypeId> = self
            .types
            .object_maps(ty)
            .map(|map| self.model.slot_type(self.model.map(map).isa))
            .collect();

        self.types.union_all(parents)
    }

    pub(crate) fn assign_entry(&mut self, base: TypeId, key: KeyRef<'_>, value: TypeId) {
        let maps: Vec<MapId> = self.types.object_maps(base).collect();

        for map in maps {
            let slot = if key.is_isa() {
                self.model.map(map).isa
            } else {
                self.model.insert_entry(map, key)
            };

            self.widen(slot, value);
        }
    }

    pub(crate) fn assign_dynamic_entry(&mut self, base: TypeId, key: TypeId, value: TypeId) {
        let maps: Vec<MapId> = self.types.object_maps(base).collect();

        for map in maps {
            let data = self.model.map(map);
            let (key_slot, value_slot) = (data.dynamic_key, data.dynamic_value);

            self.widen(key_slot, key);
            self.widen(value_slot, value);
        }
    }

    pub(crate) fn entry_values_for_key(&mut self, map: MapId, key: TypeId) -> TypeId {
        let mut accepts_strings = false;
        let mut accepts_numbers = false;

        for variant in self.types.variants(key) {
            match self.types.get(variant) {
                Type::String => accepts_strings = true,
                Type::Number => accepts_numbers = true,
                _ => {
                    accepts_strings = true;
                    accepts_numbers = true;
                }
            }
        }

        let data = self.model.map(map);
        let mut values: Vec<TypeId> = data
            .entries
            .iter()
            .filter(|(key, _)| match key {
                Key::String(_) => accepts_strings,
                Key::Number(_) => accepts_numbers,
            })
            .map(|(_, &slot)| self.model.slot_type(slot))
            .collect();

        values.push(self.model.slot_type(data.dynamic_value));

        self.types.union_all(values)
    }

    pub(crate) fn entry_types(&mut self, map: MapId) -> (TypeId, TypeId) {
        let data = self.model.map(map);
        let mut keys = vec![self.model.slot_type(data.dynamic_key)];
        let mut values = vec![self.model.slot_type(data.dynamic_value)];

        for (key, &slot) in &data.entries {
            keys.push(match key {
                Key::String(_) => TypeId::STRING,
                Key::Number(_) => TypeId::NUMBER,
            });
            values.push(self.model.slot_type(slot));
        }

        (self.types.union_all(keys), self.types.union_all(values))
    }
}
