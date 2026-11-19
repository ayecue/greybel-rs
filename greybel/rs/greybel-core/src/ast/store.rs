use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use super::{
    Atom, AtomTable, Child, DescendantIter, Node, NodeCommentContainer, NodeFieldContainer, NodeId,
    NodeKind, NodeList, NodeListStorage,
};
use crate::position::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorChain {
    pub first: NodeId,
    pub links: SmallVec<[NodeId; 4]>,
}

#[derive(Debug, Clone, Default)]
pub struct NodeStore {
    nodes: Vec<Node>,
    comments: FxHashMap<NodeId, NodeCommentContainer>,
    atoms: AtomTable,
    lists: NodeListStorage,
}

impl NodeStore {
    pub fn reserve_for_source(&mut self, source_len: usize) {
        self.nodes.reserve(source_len / 6);
        self.atoms.reserve(source_len / 64);
        self.lists.reserve(source_len / 12);
    }

    #[must_use]
    pub fn list(&self, list: NodeList) -> &[NodeId] {
        self.lists.get(list)
    }

    #[must_use]
    pub fn lists(&self) -> &NodeListStorage {
        &self.lists
    }

    pub fn push_list(&mut self, ids: &[NodeId]) -> NodeList {
        self.lists.push(ids)
    }

    pub fn list_buffer(&mut self) -> Vec<NodeId> {
        self.lists.buffer()
    }

    pub fn finish_list(&mut self, buffer: Vec<NodeId>) -> NodeList {
        self.lists.finish(buffer)
    }

    pub fn recycle_list(&mut self, buffer: Vec<NodeId>) {
        self.lists.recycle(buffer);
    }

    #[must_use]
    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.get(id).kind.children(&self.lists)
    }

    pub fn for_each_child(&self, id: NodeId, mut f: impl FnMut(NodeId)) {
        self.get(id).kind.for_each_child(&self.lists, &mut f);
    }

    pub fn insert_atom(&mut self, text: &str) -> Atom {
        self.atoms.insert(text)
    }

    pub(crate) fn insert_atom_with_hash(&mut self, text: &str, hash: u32) -> Atom {
        self.atoms.insert_with_hash(text, hash)
    }

    #[must_use]
    pub fn atoms(&self) -> &AtomTable {
        &self.atoms
    }

    #[must_use]
    pub fn resolve(&self, atom: Atom) -> &str {
        self.atoms.resolve(atom)
    }
}

impl NodeStore {
    pub fn alloc(
        &mut self,
        kind: impl Into<NodeKind>,
        span: Span,
        scope: Option<NodeId>,
    ) -> NodeId {
        let id = NodeId::from_index(self.nodes.len());
        let kind = kind.into();
        let chain_left = kind.chain_left();
        let mut depth = 0;

        kind.for_each_child(&self.lists, &mut |child| {
            let node = &self.nodes[child.index()];
            let continues_chain = Some(child) == chain_left && node.kind.chain_left().is_some();

            depth = depth.max(if continues_chain {
                node.depth.saturating_sub(1)
            } else {
                node.depth
            });
        });
        self.nodes.push(Node {
            kind,
            span,
            scope,
            depth: depth.saturating_add(1),
        });

        id
    }

    #[must_use]
    pub fn operator_chain(&self, id: NodeId) -> OperatorChain {
        let mut links = SmallVec::new();
        let mut first = id;

        while let Some(left) = self.get(first).kind.chain_left() {
            links.push(first);
            first = left;
        }

        links.reverse();

        OperatorChain { first, links }
    }

    #[must_use]
    pub fn depth(&self, id: NodeId) -> usize {
        usize::from(self.nodes[id.index()].depth)
    }

    pub fn set_kind(&mut self, id: NodeId, kind: impl Into<NodeKind>) {
        self.get_mut(id).kind = kind.into();
    }

    pub fn close(&mut self, id: NodeId, end: &Span) {
        self.get_mut(id).span.end = end.end;
    }

    pub fn close_block(&mut self, id: NodeId, body: Vec<NodeId>, end: &Span) {
        let body = self.finish_list(body);

        self.get_mut(id)
            .kind
            .as_block_mut()
            .expect("node has a body")
            .set_body(body);
        self.close(id, end);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[must_use]
    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id.index()]
    }

    pub fn get_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id.index()]
    }

    #[must_use]
    pub fn comments(&self, id: NodeId) -> Option<&NodeCommentContainer> {
        self.comments.get(&id)
    }

    pub fn comments_mut(&mut self, id: NodeId) -> &mut NodeCommentContainer {
        self.comments.entry(id).or_default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (NodeId::from_index(i), node))
    }

    pub fn clone_subtree(&mut self, id: NodeId) -> NodeId {
        let nodes: Vec<NodeId> = DescendantIter::new(self, id).collect();
        let mut copies = FxHashMap::default();

        for &original in nodes.iter().rev() {
            let node = self.get(original).clone();
            let mut kind = node.kind;

            kind.map_children(&mut |child| match child {
                Child::Node(child) => Child::Node(copies[&child]),
                Child::List(list) => {
                    let child_copies: Vec<NodeId> =
                        self.list(list).iter().map(|child| copies[child]).collect();

                    Child::List(self.push_list(&child_copies))
                }
            });

            let copy = self.alloc(kind, node.span, node.scope);

            if let Some(comments) = self.comments.get(&original).cloned() {
                self.comments.insert(copy, comments);
            }

            copies.insert(original, copy);
        }

        copies[&id]
    }
}
