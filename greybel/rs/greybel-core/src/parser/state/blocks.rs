use std::any::Any;

use crate::ast::{NodeId, NodeStore};
use crate::parser::Parser;
use crate::position::Span;

pub trait PendingBlock: Any {
    fn body_mut(&mut self) -> &mut Vec<NodeId>;

    fn body(&self) -> &[NodeId] {
        &[]
    }

    fn name(&self) -> &'static str;
    fn start(&self, nodes: &NodeStore) -> u32;
    fn complete(self: Box<Self>, parser: &mut Parser<'_>, end: &Span);
    fn awaits_end(&self, _nodes: &NodeStore) -> bool {
        true
    }
}

#[derive(Default)]
pub struct BlockStack {
    blocks: Vec<Box<dyn PendingBlock>>,
}

#[derive(Debug, Clone, Copy)]
pub struct BlockSnapshot {
    len: usize,
    body: Option<usize>,
}

impl BlockStack {
    pub fn push<P: PendingBlock>(&mut self, block: P) {
        self.blocks.push(Box::new(block));
    }

    pub fn top_mut(&mut self) -> &mut dyn PendingBlock {
        self.blocks
            .last_mut()
            .expect("chunk is always open")
            .as_mut()
    }

    pub fn top_mut_as<P: PendingBlock>(&mut self) -> Option<&mut P> {
        let block: &mut dyn PendingBlock = self.blocks.last_mut()?.as_mut();

        (block as &mut dyn Any).downcast_mut::<P>()
    }

    #[must_use]
    pub fn contains<P: PendingBlock>(&self) -> bool {
        self.blocks
            .iter()
            .any(|block| (block.as_ref() as &dyn Any).is::<P>())
    }

    #[must_use]
    pub fn top_is<P: PendingBlock>(&self) -> bool {
        self.blocks
            .last()
            .is_some_and(|block| (block.as_ref() as &dyn Any).is::<P>())
    }

    pub fn pop_as<P: PendingBlock>(&mut self) -> Option<P> {
        if !self.is_nested() || !self.top_is::<P>() {
            return None;
        }

        let block: Box<dyn Any> = self.blocks.pop()?;

        block.downcast::<P>().ok().map(|block| *block)
    }

    pub fn pop_inner(&mut self) -> Option<Box<dyn PendingBlock>> {
        if self.is_nested() {
            self.blocks.pop()
        } else {
            None
        }
    }

    pub(crate) fn pop_root(&mut self) -> Option<Box<dyn PendingBlock>> {
        self.blocks.pop()
    }

    #[must_use]
    pub fn is_nested(&self) -> bool {
        self.blocks.len() > 1
    }

    pub fn top_inner(&self) -> Option<&dyn PendingBlock> {
        if self.is_nested() {
            self.blocks.last().map(Box::as_ref)
        } else {
            None
        }
    }

    #[must_use]
    pub fn depth(&self) -> usize {
        self.blocks.len().saturating_sub(1)
    }

    /// The statements of the file so far, outside any block.
    #[must_use]
    pub fn root_statements(&self) -> &[NodeId] {
        self.blocks.first().map_or(&[], |block| block.body())
    }

    pub fn push_statement(&mut self, id: NodeId) {
        self.top_mut().body_mut().push(id);
    }

    pub fn push_statement_at(&mut self, depth: usize, id: NodeId) {
        self.blocks[depth].body_mut().push(id);
    }

    pub(crate) fn snapshot(&mut self) -> BlockSnapshot {
        BlockSnapshot {
            len: self.blocks.len(),
            body: self.blocks.last_mut().map(|block| block.body_mut().len()),
        }
    }

    pub(crate) fn restore(&mut self, snapshot: BlockSnapshot) {
        self.blocks.truncate(snapshot.len);
        if let (Some(body), Some(block)) = (snapshot.body, self.blocks.last_mut()) {
            block.body_mut().truncate(body);
        }
    }
}
