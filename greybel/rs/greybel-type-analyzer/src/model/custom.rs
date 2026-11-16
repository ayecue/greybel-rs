use greybel_core::Span;

use super::{DocumentIndex, MapId};
use crate::types::TypeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomType {
    pub name: String,
    pub map: MapId,
    pub document: DocumentIndex,
    pub span: Span,
    pub type_parameters: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericInstance {
    pub generic: MapId,
    pub arguments: Vec<TypeId>,
}
