use greybel_core::syntax::{ArrowBody, Assignment, Function};
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
    let NodeKind::Function(function) = s.kind(value) else {
        return None;
    };
    let dev_mode = s.options().dev_mode;
    let spans_lines = s.end_line(value) > s.end_line(container);

    match function.arrow {
        Some(ArrowBody::Expression) if dev_mode => None,
        Some(_) if !dev_mode => Some(value),
        _ => spans_lines.then_some(value),
    }
}

/// The parameter assignment of `parameter` when its default isn't a literal, which the function body sets instead.
pub(crate) fn expression_default<'a>(
    s: &EmitState<'a>,
    parameter: NodeId,
) -> Option<&'a Assignment> {
    let NodeKind::Assignment(assignment) = s.kind(parameter) else {
        return None;
    };

    (!s.options().dev_mode && !Function::is_literal_default(s.ast().nodes(), assignment.init))
        .then_some(assignment)
}

/// The nodes emitted for `parameters`: a parameter whose default the body sets is emitted without it.
pub(crate) fn emitted_parameters(s: &EmitState<'_>, parameters: &[NodeId]) -> Vec<NodeId> {
    parameters
        .iter()
        .map(|&parameter| {
            expression_default(s, parameter).map_or(parameter, |assignment| assignment.variable)
        })
        .collect()
}
