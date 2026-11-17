use greybel_core::syntax::BinaryOp;
use greybel_core::{Ast, Node, NodeId, NodeKind, Visitor};
use rustc_hash::FxHashSet;

/// Nodes where a concatenation written in their place needs parentheses: operands of arithmetic and unary operators,
/// except the left operand of `+` and `-`, and bases of member reads, indexes, slices and calls. Comparisons bind
/// looser than `+`, so their operands need none. Parentheses in the source are seen through.
#[derive(Debug, Default)]
pub(crate) struct OperandTable {
    operands: FxHashSet<NodeId>,
}

impl OperandTable {
    pub(crate) fn of(ast: &Ast) -> Self {
        let mut collector = OperandCollector {
            ast,
            operands: FxHashSet::default(),
        };

        ast.walk(ast.root(), &mut collector);

        Self {
            operands: collector.operands,
        }
    }

    pub(crate) fn contains(&self, id: NodeId) -> bool {
        self.operands.contains(&id)
    }
}

struct OperandCollector<'a> {
    ast: &'a Ast,
    operands: FxHashSet<NodeId>,
}

impl OperandCollector<'_> {
    fn add(&mut self, mut id: NodeId) {
        self.operands.insert(id);

        while let NodeKind::Parenthesis(node) = self.ast.kind(id) {
            id = node.expression;
            self.operands.insert(id);
        }
    }
}

impl Visitor for OperandCollector<'_> {
    fn enter(&mut self, _id: NodeId, node: &Node) -> bool {
        match &node.kind {
            NodeKind::Member(node) => self.add(node.base),
            NodeKind::Index(node) => self.add(node.base),
            NodeKind::Slice(node) => self.add(node.base),
            NodeKind::Call(node) => self.add(node.base),
            NodeKind::Unary(node) => self.add(node.argument),
            NodeKind::Spread(node) => self.add(node.argument),
            NodeKind::Binary(node) => match node.operator {
                BinaryOp::Add | BinaryOp::Subtract => self.add(node.right),
                BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo | BinaryOp::Power => {
                    self.add(node.left);
                    self.add(node.right);
                }
                _ => {}
            },
            _ => {}
        }

        true
    }
}
