use std::sync::Arc;

use rustc_hash::FxHashMap;

use super::{ApiFunction, FunctionRef, LiteralType, Type, TypeId};
use crate::model::MapId;

const MAX_DEPTH: u8 = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeTable {
    types: Vec<Type>,
    depths: Vec<u8>,
    ids: FxHashMap<Type, TypeId>,
    unions_by_hash: FxHashMap<u64, TypeId>,
    unions_by_pair: UnionCache,
    api_members: ApiMemberCache,
    api_signatures: ApiSignatureCache,
    union_builder: UnionBuilder,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedApiSignature {
    pub parameters: Arc<[TypeId]>,
    pub returns: TypeId,
}

#[derive(Debug, Default)]
struct ApiSignatureCache(FxHashMap<(usize, usize), ResolvedApiSignature>);

impl Clone for ApiSignatureCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for ApiSignatureCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for ApiSignatureCache {}

pub(crate) enum CachedApiMember {
    Unknown,
    Missing,
    Present(TypeId),
}

#[derive(Debug, Default)]
struct ApiMemberCache(FxHashMap<&'static str, FxHashMap<Box<str>, Option<TypeId>>>);

impl Clone for ApiMemberCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for ApiMemberCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for ApiMemberCache {}

#[derive(Debug, Clone, Default)]
struct UnionCache(FxHashMap<(TypeId, TypeId), TypeId>);

impl PartialEq for UnionCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for UnionCache {}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct UnionBuilder {
    has_unknown: bool,
    has_never: bool,
    variants: Vec<TypeId>,
    items: Vec<TypeId>,
    keys: Vec<TypeId>,
    values: Vec<TypeId>,
}

impl UnionBuilder {
    fn absorb(&mut self, shape: &Type, ty: TypeId) -> bool {
        match shape {
            Type::Any => return true,
            Type::Unknown => self.has_unknown = true,
            Type::Never => self.has_never = true,
            Type::List(item) => self.items.push(*item),
            Type::Map { key, value } => {
                self.keys.push(*key);
                self.values.push(*value);
            }
            _ => self.variants.push(ty),
        }

        false
    }

    /// Removes literal types whose base type is a variant too: `"tcp" or string` is `string`.
    fn remove_covered_literals(&mut self, types: &TypeTable) {
        let (strings, numbers) = (
            self.variants.contains(&TypeId::STRING),
            self.variants.contains(&TypeId::NUMBER),
        );

        if !strings && !numbers {
            return;
        }

        self.variants.retain(|&variant| match types.get(variant) {
            Type::Literal(LiteralType::String(_)) => !strings,
            Type::Literal(LiteralType::Number(_)) => !numbers,
            _ => true,
        });
    }

    fn clear(&mut self) {
        self.has_unknown = false;
        self.has_never = false;
        self.variants.clear();
        self.items.clear();
        self.keys.clear();
        self.values.clear();
    }
}

impl Default for TypeTable {
    fn default() -> Self {
        let mut table = Self {
            types: Vec::new(),
            depths: Vec::new(),
            ids: FxHashMap::default(),
            unions_by_hash: FxHashMap::default(),
            unions_by_pair: UnionCache::default(),
            api_members: ApiMemberCache::default(),
            api_signatures: ApiSignatureCache::default(),
            union_builder: UnionBuilder::default(),
        };

        for (ty, id) in [
            (Type::Unknown, TypeId::UNKNOWN),
            (Type::Any, TypeId::ANY),
            (Type::Null, TypeId::NULL),
            (Type::Number, TypeId::NUMBER),
            (Type::String, TypeId::STRING),
            (Type::Never, TypeId::NEVER),
        ] {
            assert_eq!(table.insert(ty), id);
        }

        table
    }
}

impl TypeTable {
    #[must_use]
    pub fn get(&self, id: TypeId) -> &Type {
        &self.types[id.index()]
    }

    pub fn insert(&mut self, ty: Type) -> TypeId {
        if let Some(&id) = self.ids.get(&ty) {
            return id;
        }

        let ty = match ty {
            Type::List(value) => Type::List(self.truncate_to_depth(value, MAX_DEPTH - 1)),
            Type::Map { key, value } => Type::Map {
                key: self.truncate_to_depth(key, MAX_DEPTH - 1),
                value: self.truncate_to_depth(value, MAX_DEPTH - 1),
            },
            ty => ty,
        };

        if let Some(&id) = self.ids.get(&ty) {
            return id;
        }

        let depth = match &ty {
            Type::List(value) => self.depth(*value) + 1,
            Type::Map { key, value } => self.depth(*key).max(self.depth(*value)) + 1,
            Type::Union(variants) => variants
                .iter()
                .map(|&variant| self.depth(variant))
                .max()
                .unwrap_or_default(),
            _ => 0,
        };
        let id = TypeId::new(self.types.len());

        if let Type::Union(variants) = &ty {
            self.unions_by_hash
                .entry(Self::variants_hash(variants))
                .or_insert(id);
        }

        self.types.push(ty.clone());
        self.depths.push(depth);
        self.ids.insert(ty, id);

        id
    }

    pub fn list(&mut self, value: TypeId) -> TypeId {
        self.insert(Type::List(value))
    }

    pub fn map(&mut self, key: TypeId, value: TypeId) -> TypeId {
        self.insert(Type::Map { key, value })
    }

    pub fn union(&mut self, left: TypeId, right: TypeId) -> TypeId {
        if right == TypeId::NEVER {
            return left;
        }

        if left == TypeId::NEVER {
            return right;
        }

        if left == right || right == TypeId::UNKNOWN {
            return left;
        }

        if left == TypeId::UNKNOWN {
            return right;
        }

        if left == TypeId::ANY || right == TypeId::ANY {
            return TypeId::ANY;
        }

        let pair = (left.min(right), left.max(right));

        if let Some(&union) = self.unions_by_pair.0.get(&pair) {
            return union;
        }

        let union = self.union_all([left, right]);

        self.unions_by_pair.0.insert(pair, union);

        union
    }

    pub fn union_all(&mut self, types: impl IntoIterator<Item = TypeId>) -> TypeId {
        let mut parts = std::mem::take(&mut self.union_builder);
        let union = self.union_parts(&mut parts, types);

        parts.clear();
        self.union_builder = parts;

        union
    }

    fn union_parts(
        &mut self,
        parts: &mut UnionBuilder,
        types: impl IntoIterator<Item = TypeId>,
    ) -> TypeId {
        for ty in types {
            let any = match self.get(ty) {
                Type::Union(inner) => inner
                    .iter()
                    .any(|&variant| parts.absorb(self.get(variant), variant)),
                shape => parts.absorb(shape, ty),
            };

            if any {
                return TypeId::ANY;
            }
        }

        if !parts.items.is_empty() {
            let item = self.insert_union(&parts.items);
            let list = self.list(item);

            parts.variants.push(list);
        }

        if !parts.keys.is_empty() {
            let key = self.insert_union(&parts.keys);
            let value = self.insert_union(&parts.values);
            let map = self.map(key, value);

            parts.variants.push(map);
        }

        parts.variants.sort_unstable();
        parts.variants.dedup();
        parts.remove_covered_literals(self);

        match parts.variants.as_slice() {
            [] if parts.has_never && !parts.has_unknown => TypeId::NEVER,
            [] => TypeId::UNKNOWN,
            [single] => *single,
            variants => match self.unions_by_hash.get(&Self::variants_hash(variants)) {
                Some(&union) if matches!(self.get(union), Type::Union(known) if **known == *variants) => {
                    union
                }
                _ => self.insert(Type::Union(variants.into())),
            },
        }
    }

    fn variants_hash(variants: &[TypeId]) -> u64 {
        use std::hash::BuildHasher;

        rustc_hash::FxBuildHasher.hash_one(variants)
    }

    fn insert_union(&mut self, types: &[TypeId]) -> TypeId {
        match types {
            [single] => *single,
            _ => self.union_all(types.iter().copied()),
        }
    }

    pub(crate) fn cached_api_signature(
        &self,
        function: ApiFunction,
    ) -> Option<ResolvedApiSignature> {
        self.api_signatures.0.get(&Self::api_key(function)).cloned()
    }

    pub(crate) fn cache_api_signature(
        &mut self,
        function: ApiFunction,
        types: ResolvedApiSignature,
    ) {
        self.api_signatures.0.insert(Self::api_key(function), types);
    }

    pub(crate) fn api_key(function: ApiFunction) -> (usize, usize) {
        (
            std::ptr::from_ref(function.function).addr(),
            function.origin.as_ptr().addr(),
        )
    }

    pub(crate) fn cached_api_member(&self, type_name: &'static str, name: &str) -> CachedApiMember {
        match self
            .api_members
            .0
            .get(type_name)
            .and_then(|members| members.get(name))
        {
            Some(Some(ty)) => CachedApiMember::Present(*ty),
            Some(None) => CachedApiMember::Missing,
            None => CachedApiMember::Unknown,
        }
    }

    pub(crate) fn cache_api_member(
        &mut self,
        type_name: &'static str,
        name: &str,
        ty: Option<TypeId>,
    ) {
        self.api_members
            .0
            .entry(type_name)
            .or_default()
            .insert(name.into(), ty);
    }

    #[must_use]
    pub fn is_union(&self, ty: TypeId) -> bool {
        matches!(self.get(ty), Type::Union(_))
    }

    /// `ty` with a literal type as its base type (`"tcp"` as `string`); other types as they are.
    #[must_use]
    pub fn without_literal(&self, ty: TypeId) -> TypeId {
        match self.get(ty) {
            Type::Literal(literal) => literal.base(),
            _ => ty,
        }
    }

    pub fn variants(&self, ty: TypeId) -> impl Iterator<Item = TypeId> + '_ {
        let (union, single) = match self.get(ty) {
            Type::Union(variants) => (&variants[..], None),
            _ => (&[][..], Some(ty)),
        };

        union.iter().copied().chain(single)
    }

    pub fn object_maps(&self, ty: TypeId) -> impl Iterator<Item = MapId> + '_ {
        self.variants(ty)
            .filter_map(|variant| match self.get(variant) {
                Type::Object(map) => Some(*map),
                _ => None,
            })
    }

    #[must_use]
    pub fn object_map(&self, ty: TypeId) -> Option<MapId> {
        match self.get(ty) {
            Type::Object(map) => Some(*map),
            _ => None,
        }
    }

    pub fn function_refs(&self, ty: TypeId) -> impl Iterator<Item = FunctionRef> + '_ {
        self.variants(ty)
            .filter_map(|variant| match self.get(variant) {
                Type::Function(function) => Some(*function),
                _ => None,
            })
    }

    pub(crate) fn contains(&self, ty: TypeId, predicate: &impl Fn(&Type) -> bool) -> bool {
        let shape = self.get(ty);

        predicate(shape)
            || match shape {
                Type::List(item) => self.contains(*item, predicate),
                Type::Map { key, value } => {
                    self.contains(*key, predicate) || self.contains(*value, predicate)
                }
                Type::Union(variants) => variants
                    .iter()
                    .any(|&variant| self.contains(variant, predicate)),
                _ => false,
            }
    }

    pub fn type_parameter(&mut self, name: &str) -> TypeId {
        self.insert(Type::TypeParameter {
            name: name.into(),
            bound: TypeId::ANY,
        })
    }

    pub fn function(&mut self, function: FunctionRef) -> TypeId {
        self.insert(Type::Function(function))
    }

    pub fn erase_type_parameters(&mut self, ty: TypeId) -> TypeId {
        if !self
            .variants(ty)
            .any(|variant| matches!(self.get(variant), Type::TypeParameter { .. }))
        {
            return ty;
        }

        let variants: Vec<TypeId> = self
            .variants(ty)
            .map(|variant| match self.get(variant) {
                Type::TypeParameter { bound, .. } => *bound,
                _ => variant,
            })
            .collect();
        let variants: Vec<TypeId> = variants
            .into_iter()
            .map(|variant| self.erase_type_parameters(variant))
            .collect();

        self.union_all(variants)
    }

    fn depth(&self, id: TypeId) -> u8 {
        self.depths[id.index()]
    }

    fn truncate_to_depth(&mut self, id: TypeId, depth: u8) -> TypeId {
        if self.depth(id) <= depth {
            return id;
        }

        match self.get(id).clone() {
            _ if depth == 0 => TypeId::ANY,
            Type::List(value) => {
                let value = self.truncate_to_depth(value, depth - 1);

                self.insert(Type::List(value))
            }
            Type::Map { key, value } => {
                let key = self.truncate_to_depth(key, depth - 1);
                let value = self.truncate_to_depth(value, depth - 1);

                self.insert(Type::Map { key, value })
            }
            Type::Union(variants) => {
                let variants: Vec<TypeId> = variants
                    .iter()
                    .map(|&variant| self.truncate_to_depth(variant, depth))
                    .collect();

                self.union_all(variants)
            }
            _ => id,
        }
    }
}

#[cfg(test)]
#[path = "tests/table.rs"]
mod tests;
