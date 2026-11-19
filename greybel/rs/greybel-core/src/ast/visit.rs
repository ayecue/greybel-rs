use super::{Node, NodeId, NodeStore};

pub trait Visitor {
    fn enter(&mut self, _id: NodeId, _node: &Node) -> bool {
        true
    }

    fn exit(&mut self, _id: NodeId, _node: &Node) {}
}

pub struct DescendantIter<'a> {
    nodes: &'a NodeStore,
    stack: Vec<NodeId>,
    current: Option<NodeId>,
}

impl<'a> DescendantIter<'a> {
    #[must_use]
    pub fn new(nodes: &'a NodeStore, id: NodeId) -> Self {
        Self {
            nodes,
            stack: vec![id],
            current: None,
        }
    }

    pub fn skip_children(&mut self) {
        self.current = None;
    }
}

impl Iterator for DescendantIter<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        if let Some(current) = self.current.take() {
            let start = self.stack.len();

            self.nodes
                .for_each_child(current, |child| self.stack.push(child));
            self.stack[start..].reverse();
        }

        self.current = self.stack.pop();

        self.current
    }
}

const RECURSION_LIMIT: usize = 64;

pub fn walk(nodes: &NodeStore, id: NodeId, visitor: &mut impl Visitor) {
    walk_from(nodes, id, visitor, 0);
}

fn walk_from(nodes: &NodeStore, id: NodeId, visitor: &mut impl Visitor, depth: usize) {
    if depth == RECURSION_LIMIT {
        return walk_with_stack(nodes, id, visitor);
    }

    let node = nodes.get(id);

    if visitor.enter(id, node) {
        nodes.for_each_child(id, |child| walk_from(nodes, child, visitor, depth + 1));
    }

    visitor.exit(id, node);
}

fn walk_with_stack(nodes: &NodeStore, id: NodeId, visitor: &mut impl Visitor) {
    enum Step {
        Enter(NodeId),
        Exit(NodeId),
    }

    let mut stack = vec![Step::Enter(id)];

    while let Some(step) = stack.pop() {
        match step {
            Step::Enter(id) => {
                let node = nodes.get(id);

                stack.push(Step::Exit(id));

                if visitor.enter(id, node) {
                    let start = stack.len();

                    nodes.for_each_child(id, |child| stack.push(Step::Enter(child)));
                    stack[start..].reverse();
                }
            }
            Step::Exit(id) => visitor.exit(id, nodes.get(id)),
        }
    }
}
