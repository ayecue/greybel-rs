use greybel_symbols::names::SELF;
use meta_utils::{BaseType, LocalizedDefinition, TypeLookup, TypeMeta};

use crate::api::Api;
use crate::model::{Key, KeyRef, MapId, MapOrigin, SemanticModel, SlotId, SlotList};
use crate::types::{ApiFunction, CachedApiMember, FunctionRef, Type, TypeId, TypeTable};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub name: String,
    pub ty: TypeId,
    pub from_api: bool,
}

pub(crate) struct Lookup<'a> {
    types: &'a mut TypeTable,
    model: &'a SemanticModel,
    api: Api,
}

impl<'a> Lookup<'a> {
    pub(crate) fn new(types: &'a mut TypeTable, model: &'a SemanticModel, api: Api) -> Self {
        Self { types, model, api }
    }

    pub(crate) fn member_type(&mut self, ty: TypeId, key: KeyRef<'_>) -> TypeId {
        let ty = self.types.erase_type_parameters(ty);

        self.map_variants(ty, |this, variant| this.variant_member_type(variant, key))
    }

    fn map_variants(
        &mut self,
        ty: TypeId,
        mut each: impl FnMut(&mut Self, TypeId) -> TypeId,
    ) -> TypeId {
        if !self.types.is_union(ty) {
            return each(self, ty);
        }

        let variants: Vec<TypeId> = self.types.variants(ty).collect();
        let results: Vec<TypeId> = variants
            .into_iter()
            .map(|variant| each(self, variant))
            .collect();

        self.types.union_all(results)
    }

    fn variant_member_type(&mut self, ty: TypeId, key: KeyRef<'_>) -> TypeId {
        let name = key.as_str();

        match *self.types.get(ty) {
            Type::Unknown | Type::Null | Type::Union(_) | Type::TypeParameter { .. } => {
                TypeId::UNKNOWN
            }
            Type::Any => name
                .and_then(|name| self.api_member_type(BaseType::Any.as_str(), name))
                .unwrap_or(TypeId::ANY),
            Type::Class(class) => name
                .and_then(|name| self.api_member_type(class, name))
                .or_else(|| self.prototype_entry(BaseType::Map, key))
                .unwrap_or(TypeId::UNKNOWN),
            Type::Object(map) => self.object_member_type(map, key),
            ref other => {
                let base = other.base_type().expect("built-in types have a base");

                self.base_member_type(base, key)
            }
        }
    }

    fn base_member_type(&mut self, base: BaseType, key: KeyRef<'_>) -> TypeId {
        if let Some(member_type) = self.prototype_entry(base, key) {
            return member_type;
        }

        key.as_str()
            .and_then(|name| self.api_member_type(base.as_str(), name))
            .unwrap_or(TypeId::UNKNOWN)
    }

    fn prototype_entry(&self, base: BaseType, key: KeyRef<'_>) -> Option<TypeId> {
        let map = self.model.prototype(base)?;
        let slot = self.model.map(map).own_entry(key)?;

        self.entry_type(slot, key)
    }

    fn entry_type(&self, slot: SlotId, key: KeyRef<'_>) -> Option<TypeId> {
        if key.is_isa() {
            self.model.known_slot_type(slot)
        } else {
            Some(self.model.slot_type(slot))
        }
    }

    fn object_member_type(&mut self, map: MapId, key: KeyRef<'_>) -> TypeId {
        let member_type = match self.model.visible_entry_slots(self.types, map, key) {
            SlotList::Empty => None,
            SlotList::One(slot) => self.entry_type(slot, key),
            SlotList::Many(slots) => {
                let member_types: Vec<TypeId> = slots
                    .into_iter()
                    .filter_map(|slot| self.entry_type(slot, key))
                    .collect();

                (!member_types.is_empty()).then(|| self.types.union_all(member_types))
            }
        };

        if let Some(member_type) = member_type {
            return member_type;
        }

        if let (MapOrigin::Prototype(base), Some(name)) = (self.model.map(map).origin, key.as_str())
            && let Some(member_type) = self.api_member_type(base.as_str(), name)
        {
            return member_type;
        }

        self.base_member_type(BaseType::Map, key)
    }

    pub(crate) fn entry_slots(&self, ty: TypeId, key: KeyRef<'_>) -> SlotList {
        let mut slots = SlotList::Empty;

        for variant in self.types.variants(ty) {
            let ty = self.types.get(variant);

            if let Type::Object(map) = ty {
                slots.extend(&self.model.visible_entry_slots(self.types, *map, key));
                continue;
            }

            if let Some(slot) = ty
                .base_type()
                .and_then(|base| self.model.prototype(base))
                .and_then(|map| self.model.map(map).entries.get(&key).copied())
            {
                slots.add(slot);
            }
        }

        slots.sort_by_id()
    }

    pub(crate) fn general_function_type(&mut self, name: &str) -> Option<TypeId> {
        self.api_member_type(BaseType::General.as_str(), name)
    }

    fn api_member_type(&mut self, type_name: &'static str, name: &str) -> Option<TypeId> {
        match self.types.cached_api_member(type_name, name) {
            CachedApiMember::Present(ty) => return Some(ty),
            CachedApiMember::Missing => return None,
            CachedApiMember::Unknown => {}
        }

        let ty = self
            .api
            .member(type_name, name)
            .map(|member| self.api_definition_type(member));

        self.types.cache_api_member(type_name, name, ty);

        ty
    }

    fn api_definition_type(&mut self, member: LocalizedDefinition<'static>) -> TypeId {
        if let Some(function) = member.definition().as_function() {
            return self.types.function(FunctionRef::Api(ApiFunction::new(
                member.origin(),
                member.name(),
                function,
            )));
        }

        self.api_type(member.definition().type_meta())
    }

    pub(crate) fn invocation_type(&mut self, ty: TypeId) -> TypeId {
        self.map_variants(ty, |this, variant| match *this.types.get(variant) {
            Type::Function(function) => this.return_type(function),
            _ => variant,
        })
    }

    pub(crate) fn return_type(&mut self, function: FunctionRef) -> TypeId {
        match function {
            FunctionRef::Any => TypeId::ANY,
            FunctionRef::Api(function) => {
                let returns = function.function().returns();

                if returns.is_empty() {
                    return TypeId::ANY;
                }

                let returns: Vec<TypeId> = returns
                    .iter()
                    .map(|meta| self.api_type_in(meta, function.origin))
                    .collect();

                self.types.union_all(returns)
            }
            FunctionRef::Body(function) => self
                .model
                .slot_type(self.model.function(function).return_slot),
            FunctionRef::Signature(function) => self.model.declared_function(function).return_type,
            FunctionRef::ApiMethod(method) => self.model.api_method(method).returns,
        }
    }

    pub(crate) fn api_type(&mut self, meta: &TypeMeta) -> TypeId {
        self.api_type_with(meta, None)
    }

    fn api_type_in(&mut self, meta: &TypeMeta, origin: &str) -> TypeId {
        let parameters = self.api.type_parameters(origin);

        self.api_type_with(meta, Some(parameters))
    }

    fn api_type_with(&mut self, meta: &TypeMeta, parameters: Option<&[String]>) -> TypeId {
        match meta {
            TypeMeta::List(value) => {
                let value = self.api_type_with(value, parameters);

                self.types.list(value)
            }
            TypeMeta::Map { key, value } => {
                let key = self.api_type_with(key, parameters);
                let value = self.api_type_with(value, parameters);

                self.types.map(key, value)
            }
            TypeMeta::Name(name)
                if parameters
                    .is_some_and(|parameters| name == SELF || parameters.contains(name)) =>
            {
                self.types.type_parameter(name)
            }
            TypeMeta::Name(name) => match self.api.lookup_type(name) {
                TypeLookup::Base(BaseType::Number) => TypeId::NUMBER,
                TypeLookup::Base(BaseType::String) => TypeId::STRING,
                TypeLookup::Base(BaseType::Function) => self.types.function(FunctionRef::Any),
                TypeLookup::Base(BaseType::List) => self.types.list(TypeId::ANY),
                TypeLookup::Base(BaseType::Map) => self.types.map(TypeId::ANY, TypeId::ANY),
                TypeLookup::Base(BaseType::Any | BaseType::General) | TypeLookup::Unknown => {
                    TypeId::ANY
                }
                TypeLookup::Null => TypeId::NULL,
                TypeLookup::Class(signature) => {
                    self.types.insert(Type::Class(signature.type_name()))
                }
            },
        }
    }

    pub(crate) fn api_parameter_types(&mut self, function: ApiFunction) -> Vec<TypeId> {
        function
            .function()
            .arguments()
            .iter()
            .map(|argument| {
                let types: Vec<TypeId> = argument
                    .types()
                    .iter()
                    .map(|meta| self.api_type_in(meta, function.origin))
                    .collect();

                self.types.union_all(types)
            })
            .collect()
    }

    pub(crate) fn api_receiver_type(&mut self, function: ApiFunction) -> Option<TypeId> {
        let parameters: Vec<TypeId> = self
            .api
            .type_parameters(function.origin)
            .iter()
            .map(|name| self.types.type_parameter(name))
            .collect();

        match (BaseType::from_name(function.origin)?, &parameters[..]) {
            (BaseType::List, [item]) => Some(self.types.list(*item)),
            (BaseType::Map, [key, value]) => Some(self.types.map(*key, *value)),
            _ => None,
        }
    }

    pub(crate) fn api_mutation(
        &mut self,
        function: ApiFunction,
    ) -> Option<Vec<(Box<str>, TypeId)>> {
        let mutation = function.function().mutation()?;

        Some(
            mutation
                .fixed_types()
                .iter()
                .map(|(name, meta)| {
                    (
                        name.as_str().into(),
                        self.api_type_in(meta, function.origin),
                    )
                })
                .collect(),
        )
    }

    pub(crate) fn members(&mut self, ty: TypeId) -> Vec<Member> {
        let mut members: Vec<Member> = Vec::new();
        let ty = self.types.erase_type_parameters(ty);
        let variants: Vec<TypeId> = self.types.variants(ty).collect();

        for variant in variants {
            let (maps, api_types) = match self.types.get(variant).clone() {
                Type::Object(map) => {
                    let mut maps = self.model.isa_chain(self.types, map);
                    let api_types = match self.model.map(map).origin {
                        MapOrigin::Prototype(base) => vec![base.as_str(), BaseType::Map.as_str()],
                        _ => vec![BaseType::Map.as_str()],
                    };

                    maps.extend(self.model.prototype(BaseType::Map));

                    (maps, api_types)
                }
                Type::Class(class) => (
                    self.model.prototype(BaseType::Map).into_iter().collect(),
                    vec![class],
                ),
                Type::Any | Type::Unknown => (Vec::new(), vec![BaseType::Any.as_str()]),
                other => {
                    let Some(base) = other.base_type() else {
                        continue;
                    };

                    (
                        self.model.prototype(base).into_iter().collect(),
                        vec![base.as_str()],
                    )
                }
            };
            let mut own: Vec<Member> = Vec::new();

            for map in maps {
                for (key, &slot) in &self.model.map(map).entries {
                    let Key::String(name) = key else {
                        continue;
                    };

                    if !own.iter().any(|member| *member.name == **name) {
                        own.push(Member {
                            name: name.to_string(),
                            ty: self.model.slot_type(slot),
                            from_api: false,
                        });
                    }
                }
            }

            for api_type in api_types {
                for api_member in self.api.members(api_type) {
                    if !own.iter().any(|member| member.name == api_member.name()) {
                        let ty = self.api_definition_type(api_member);

                        own.push(Member {
                            name: api_member.name().to_owned(),
                            ty,
                            from_api: true,
                        });
                    }
                }
            }

            for member in own {
                match members.iter_mut().find(|it| it.name == member.name) {
                    Some(item) => item.ty = self.types.union(item.ty, member.ty),
                    None => members.push(member),
                }
            }
        }

        members
    }
}
