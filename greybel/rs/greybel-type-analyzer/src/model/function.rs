use greybel_core::NodeId;
use greybel_symbols::names::SELF;

use super::{CallContextId, DeclaredId, ScopeId, Site, SlotId};
use crate::annotation::DocComment;
use crate::types::{ApiFunction, TypeId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionData {
    pub site: Site,
    pub context: Option<CallContextId>,
    pub outer_scope: ScopeId,
    pub parameters: Vec<Parameter>,
    pub return_slot: SlotId,
    pub receiver_slot: SlotId,
    pub doc: Option<DocComment>,
}

impl FunctionData {
    pub(crate) fn parameters(&self, method: bool) -> &[Parameter] {
        match self.parameters.split_first() {
            Some((first, remainder)) if method && first.name == SELF => remainder,
            _ => &self.parameters,
        }
    }
}

/// API function specialized for a receiver by substituting API generic parameters and `self` in parameter and
/// return types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiMethod {
    pub function: ApiFunction,
    pub parameters: Box<[TypeId]>,
    pub returns: TypeId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub node: NodeId,
    pub slot: SlotId,
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredFunction {
    pub name: String,
    pub parameters: Vec<DeclaredParameter>,
    pub return_type: TypeId,
    pub mutated_parameter: Option<String>,
    pub generic_source: Option<DeclaredId>,
    pub description: Option<String>,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredParameter {
    pub name: String,
    pub ty: TypeId,
    pub description: Option<String>,
}
