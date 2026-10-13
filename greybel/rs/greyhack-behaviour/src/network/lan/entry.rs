use super::{LanNode, LanNodeId};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Entry<K> {
    pub node: LanNode<K>,
    pub parent: Option<LanNodeId>,
    pub children: Vec<LanNodeId>,
}
