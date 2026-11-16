use greybel_core::syntax::{Assignment, Binary};
use greybel_core::{NodeId, NodeKind};

use super::SHORTHAND_OPERATORS;
use crate::emit::EmitState;

impl<'a> EmitState<'a> {
    pub(super) fn is_block_statement(&self, id: NodeId) -> bool {
        match self.kind(id) {
            NodeKind::For(_) | NodeKind::While(_) | NodeKind::If(_) => true,
            NodeKind::Assignment(node) => matches!(self.kind(node.init), NodeKind::Function(_)),
            _ => false,
        }
    }

    pub(super) fn count_operations(&self, id: NodeId) -> usize {
        let id = self.unwrap_from_parens(id);

        if let NodeKind::ComparisonGroup(group) = self.kind(id) {
            return group.expressions.len();
        }

        let mut stack = vec![id];
        let mut count = 0;

        while let Some(current) = stack.pop() {
            let (left, right) = match self.kind(current) {
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

            stack.push(self.unwrap_from_parens(left));
            stack.push(self.unwrap_from_parens(right));
        }

        count
    }

    pub(super) fn shorthand(&self, node: &Assignment) -> Option<&'a Binary> {
        if !self.options().shortens_assignments() {
            return None;
        }

        let NodeKind::Binary(binary) = self.kind(node.init) else {
            return None;
        };
        let is_target = |id| matches!(self.kind(id), NodeKind::Identifier(_) | NodeKind::Member(_));

        (is_target(node.variable)
            && is_target(binary.left)
            && SHORTHAND_OPERATORS.contains(&binary.operator)
            && self.same_target(node.variable, binary.left))
        .then_some(binary)
    }

    fn same_target(&self, left: NodeId, right: NodeId) -> bool {
        let (left, right) = (
            self.unwrap_from_parens(left),
            self.unwrap_from_parens(right),
        );

        match (self.kind(left), self.kind(right)) {
            (NodeKind::Identifier(left_node), NodeKind::Identifier(right_node)) => {
                self.resolve(left_node.name) == self.resolve(right_node.name)
            }
            (NodeKind::Member(left_node), NodeKind::Member(right_node)) => {
                self.same_target(left_node.base, right_node.base)
                    && self.same_target(left_node.identifier, right_node.identifier)
            }
            (NodeKind::Index(left_node), NodeKind::Index(right_node)) => {
                self.same_target(left_node.base, right_node.base)
                    && self.same_target(left_node.index, right_node.index)
            }
            _ => self.text(left) == self.text(right),
        }
    }
}
