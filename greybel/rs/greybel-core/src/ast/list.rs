use std::ops::Range;

use super::NodeId;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct NodeList {
    start: u32,
    len: u32,
}

impl NodeList {
    pub const EMPTY: Self = Self { start: 0, len: 0 };

    #[must_use]
    pub fn len(self) -> usize {
        self.len as usize
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len == 0
    }

    fn range(self) -> Range<usize> {
        let start = self.start as usize;

        start..start + self.len as usize
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeListStorage {
    ids: Vec<NodeId>,
    buffers: Vec<Vec<NodeId>>,
}

impl NodeListStorage {
    pub fn reserve(&mut self, additional: usize) {
        self.ids.reserve(additional);
    }

    #[must_use]
    pub fn get(&self, list: NodeList) -> &[NodeId] {
        &self.ids[list.range()]
    }

    pub fn push(&mut self, ids: &[NodeId]) -> NodeList {
        if ids.is_empty() {
            return NodeList::EMPTY;
        }

        let start = u32::try_from(self.ids.len()).expect("fewer than 2^32 list entries");
        let len = u32::try_from(ids.len()).expect("fewer than 2^32 list entries");

        self.ids.extend_from_slice(ids);

        NodeList { start, len }
    }

    pub fn buffer(&mut self) -> Vec<NodeId> {
        self.buffers.pop().unwrap_or_default()
    }

    pub fn finish(&mut self, buffer: Vec<NodeId>) -> NodeList {
        let list = self.push(&buffer);

        self.recycle(buffer);

        list
    }

    pub fn recycle(&mut self, mut buffer: Vec<NodeId>) {
        buffer.clear();
        self.buffers.push(buffer);
    }
}
