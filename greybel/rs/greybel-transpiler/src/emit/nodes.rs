use greybel_core::{NodeId, NodeKind};

use super::EmitState;
use crate::documents::ModuleId;

pub(crate) fn unwrap_from_parens(s: &EmitState<'_>, mut id: NodeId) -> NodeId {
    while let NodeKind::Parenthesis(node) = s.kind(id) {
        id = node.expression;
    }

    id
}

pub(crate) fn unwrap_value(s: &EmitState<'_>, id: NodeId) -> NodeId {
    match s.kind(id) {
        NodeKind::ListItem(node) => node.value,
        NodeKind::MapEntry(node) => node.value,
        _ => id,
    }
}

#[must_use]
pub fn dependency_module(s: &EmitState<'_>, id: NodeId) -> Option<ModuleId> {
    if s.options().dev_mode {
        return None;
    }

    s.module()?.dependency(id)
}

pub(crate) fn deferred_function(
    s: &EmitState<'_>,
    item: NodeId,
    container: NodeId,
) -> Option<NodeId> {
    let value = unwrap_value(s, item);

    (matches!(s.kind(value), NodeKind::Function(_)) && s.end_line(value) > s.end_line(container))
        .then_some(value)
}
