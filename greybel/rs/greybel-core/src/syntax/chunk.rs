use crate::ast::{
    AstNode, DumpContext, NodeFieldContainer, NodeId, NodeList, NodeStore, ScopeData,
};
use crate::parser::{Parser, PendingBlock};
use crate::position::Span;

#[derive(Debug, Clone, Default, PartialEq, NodeFieldContainer)]
pub struct Chunk {
    #[child]
    #[body]
    pub body: NodeList,
    #[scope]
    pub scope: ScopeData,
}

impl AstNode for Chunk {
    fn type_name(&self) -> &'static str {
        "Chunk"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();
        let loc = ctx.location();

        if self.body.is_empty() {
            format!("{type_name}[{loc}][]")
        } else {
            format!("{type_name}[{loc}][\n{}\n]", ctx.block(self.body))
        }
    }
}

pub struct PendingChunk {
    pub node: NodeId,
    pub body: Vec<NodeId>,
}

impl PendingBlock for PendingChunk {
    fn body_mut(&mut self) -> &mut Vec<NodeId> {
        &mut self.body
    }

    fn name(&self) -> &'static str {
        "chunk"
    }

    fn start(&self, nodes: &NodeStore) -> u32 {
        nodes.get(self.node).span.start
    }

    fn complete(self: Box<Self>, parser: &mut Parser<'_>, end: &Span) {
        parser.stream.nodes.close_block(self.node, self.body, end);
    }
}
