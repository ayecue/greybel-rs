mod call_context;
mod custom;
mod function;
mod functions;
mod ids;
mod map;
mod maps;
mod scopes;
mod signature;
mod slots;

pub use call_context::CallContext;
pub use custom::{CustomType, GenericInstance};
pub use function::{ApiMethod, DeclaredFunction, DeclaredParameter, FunctionData, Parameter};
pub use ids::{
    ApiMethodId, CallContextId, DeclaredId, DocumentIndex, FunctionId, GenericInstanceId, MapId,
    NameId, ScopeId, Site, SlotId,
};
use indexmap::IndexMap;
pub use map::{Key, KeyRef, MapData, MapOrigin};
use rustc_hash::{FxBuildHasher, FxHashMap};
pub use signature::Documentation;
pub(crate) use signature::{FunctionSignature, SignatureParameter, SignatureType};
pub(crate) use slots::SlotList;

use crate::types::TypeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticModel {
    slots: Vec<Option<TypeId>>,
    maps: Vec<MapData>,
    map_ids: FxHashMap<(MapOrigin, Option<CallContextId>), MapId>,
    functions: Vec<FunctionData>,
    function_ids: FxHashMap<(Site, Option<CallContextId>), FunctionId>,
    variables: Vec<IndexMap<NameId, SlotId, FxBuildHasher>>,
    names: Vec<String>,
    name_ids: FxHashMap<String, NameId>,
    custom_types: IndexMap<String, CustomType, FxBuildHasher>,
    declared_functions: Vec<DeclaredFunction>,
    declared_instance_ids: FxHashMap<(DeclaredId, Vec<TypeId>), DeclaredId>,
    signature_ids: FxHashMap<(Vec<String>, Vec<TypeId>, TypeId), DeclaredId>,
    api_methods: Vec<ApiMethod>,
    api_method_ids: FxHashMap<ApiMethod, ApiMethodId>,
    generic_instances: Vec<GenericInstance>,
    generic_instance_ids: FxHashMap<GenericInstance, GenericInstanceId>,
    call_contexts: Vec<CallContext>,
    call_context_ids: FxHashMap<CallContext, CallContextId>,
    call_context_counts: FxHashMap<FunctionId, usize>,
    singleton_types: FxHashMap<&'static str, TypeId>,
    guard_types: FxHashMap<FunctionId, TypeId>,
}

impl Default for SemanticModel {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            maps: Vec::new(),
            map_ids: FxHashMap::default(),
            functions: Vec::new(),
            function_ids: FxHashMap::default(),
            variables: vec![IndexMap::default()],
            names: Vec::new(),
            name_ids: FxHashMap::default(),
            custom_types: IndexMap::default(),
            declared_functions: Vec::new(),
            declared_instance_ids: FxHashMap::default(),
            signature_ids: FxHashMap::default(),
            api_methods: Vec::new(),
            api_method_ids: FxHashMap::default(),
            generic_instances: Vec::new(),
            generic_instance_ids: FxHashMap::default(),
            call_contexts: Vec::new(),
            call_context_ids: FxHashMap::default(),
            call_context_counts: FxHashMap::default(),
            singleton_types: FxHashMap::default(),
            guard_types: FxHashMap::default(),
        }
    }
}
