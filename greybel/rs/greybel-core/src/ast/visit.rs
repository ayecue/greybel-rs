use super::{Node, NodeId, NodeStore};

pub trait Visitor {
    fn enter(&mut self, _id: NodeId, _node: &Node) -> bool {
        true
    }

    fn exit(&mut self, _id: NodeId, _node: &Node) {}
}

pub fn walk(nodes: &NodeStore, id: NodeId, visitor: &mut impl Visitor) {
    let node = nodes.get(id);

    if visitor.enter(id, node) {
        nodes.for_each_child(id, |child| walk(nodes, child, visitor));
    }

    visitor.exit(id, node);
}
