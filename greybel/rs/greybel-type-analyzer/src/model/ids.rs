use greybel_core::NodeId;

use crate::index::index_type;

index_type! {
    SlotId: "slots";
    MapId: "maps";
    FunctionId: "functions";
    DeclaredId: "declared functions";
    ApiMethodId: "API methods";
    NameId: "names";
    GenericInstanceId: "specializations";
    CallContextId: "call contexts";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeId {
    Global,
    Function(FunctionId),
}

impl ScopeId {
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Global => 0,
            Self::Function(function) => function.index() + 1,
        }
    }
}

pub type DocumentIndex = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Site {
    pub document: DocumentIndex,
    pub node: NodeId,
}
