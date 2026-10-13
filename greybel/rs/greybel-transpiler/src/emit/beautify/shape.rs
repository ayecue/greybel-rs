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

fn same_target(s: &EmitState<'_>, a: NodeId, b: NodeId) -> bool {
    let (a, b) = (unwrap_from_parens(s, a), unwrap_from_parens(s, b));

    match (s.kind(a), s.kind(b)) {
        (NodeKind::Identifier(x), NodeKind::Identifier(y)) => {
            s.resolve(x.name) == s.resolve(y.name)
        }
        (NodeKind::Member(x), NodeKind::Member(y)) => {
            same_target(s, x.base, y.base) && same_target(s, x.identifier, y.identifier)
        }
        (NodeKind::Index(x), NodeKind::Index(y)) => {
            same_target(s, x.base, y.base) && same_target(s, x.index, y.index)
        }
        _ => s.text(a) == s.text(b),
    }
}
