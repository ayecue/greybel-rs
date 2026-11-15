use greybel_core::syntax::{Assignment, Binary};
use greybel_core::{NodeId, NodeKind};

use super::SHORTHAND_OPERATORS;
use crate::emit::EmitState;
use crate::emit::nodes::unwrap_from_parens;

pub(super) fn is_block_statement(s: &EmitState<'_>, id: NodeId) -> bool {
    match s.kind(id) {
        NodeKind::For(_) | NodeKind::While(_) | NodeKind::If(_) => true,
        NodeKind::Assignment(node) => matches!(s.kind(node.init), NodeKind::Function(_)),
        _ => false,
    }
}

pub(super) fn count_operations(s: &EmitState<'_>, id: NodeId) -> usize {
    let id = unwrap_from_parens(s, id);

    if let NodeKind::ComparisonGroup(group) = s.kind(id) {
        return group.expressions.len();
    }

    let mut stack = vec![id];
    let mut count = 0;

    while let Some(current) = stack.pop() {
        let (left, right) = match s.kind(current) {
            NodeKind::Binary(node) => {
                count += 1;

                (node.left, node.right)
            }
            NodeKind::Bitwise(node) => {
                count += 1;

                (node.left, node.right)
            }
            NodeKind::Logical(node) => (node.left, node.right),
            NodeKind::Isa(node) => (node.left, node.right),
            _ => continue,
        };

        stack.push(unwrap_from_parens(s, left));
        stack.push(unwrap_from_parens(s, right));
    }

    count
}

pub(super) fn shorthand<'a>(s: &EmitState<'a>, node: &Assignment) -> Option<&'a Binary> {
    if !s.options().shortens_assignments() {
        return None;
    }

    let NodeKind::Binary(binary) = s.kind(node.init) else {
        return None;
    };
    let is_target = |id| matches!(s.kind(id), NodeKind::Identifier(_) | NodeKind::Member(_));

    (is_target(node.variable)
        && is_target(binary.left)
        && SHORTHAND_OPERATORS.contains(&binary.operator)
        && same_target(s, node.variable, binary.left))
    .then_some(binary)
}

fn same_target(state: &EmitState<'_>, left: NodeId, right: NodeId) -> bool {
    let (left, right) = (
        unwrap_from_parens(state, left),
        unwrap_from_parens(state, right),
    );

    match (state.kind(left), state.kind(right)) {
        (NodeKind::Identifier(left_node), NodeKind::Identifier(right_node)) => {
            state.resolve(left_node.name) == state.resolve(right_node.name)
        }
        (NodeKind::Member(left_node), NodeKind::Member(right_node)) => {
            same_target(state, left_node.base, right_node.base)
                && same_target(state, left_node.identifier, right_node.identifier)
        }
        (NodeKind::Index(left_node), NodeKind::Index(right_node)) => {
            same_target(state, left_node.base, right_node.base)
                && same_target(state, left_node.index, right_node.index)
        }
        _ => state.text(left) == state.text(right),
    }
}
