use std::marker::PhantomData;

use crate::ast::{AstNode, NodeId, NodeStore};
use crate::error::ParseResult;
use crate::lexer::Keyword;
use crate::parser::{KeywordStatement, Parser, PendingBlock};
use crate::position::Span;

pub trait LoopNode: AstNode + 'static {
    const NAME: &'static str;
    const END: Keyword;
}

pub struct PendingLoop<L> {
    pub node: NodeId,
    pub body: Vec<NodeId>,
    _loop: PhantomData<fn() -> L>,
}

impl<L: LoopNode> PendingLoop<L> {
    #[must_use]
    pub fn new(node: NodeId, body: Vec<NodeId>) -> Self {
        Self {
            node,
            body,
            _loop: PhantomData,
        }
    }

    pub fn open(parser: &mut Parser<'_>, node: NodeId) {
        parser.attach_pending_comments(node);

        let body = parser.stream.nodes.list_buffer();

        parser.stream.blocks.push(Self::new(node, body));
        parser.stream.scopes.enter_loop(node);
    }
}

impl<L: LoopNode> PendingBlock for PendingLoop<L> {
    fn body_mut(&mut self) -> &mut Vec<NodeId> {
        &mut self.body
    }

    fn name(&self) -> &'static str {
        L::NAME
    }

    fn start(&self, nodes: &NodeStore) -> u32 {
        nodes.get(self.node).span.start
    }

    fn complete(self: Box<Self>, parser: &mut Parser<'_>, end: &Span) {
        parser.stream.nodes.close_block(self.node, self.body, end);
        parser.register_line_item(self.node);
    }
}

pub struct EndLoopRule<L> {
    keywords: [Keyword; 1],
    _loop: PhantomData<fn() -> L>,
}

impl<L: LoopNode> EndLoopRule<L> {
    pub const RULE: Self = Self {
        keywords: [L::END],
        _loop: PhantomData,
    };
}

impl<L: LoopNode> KeywordStatement for EndLoopRule<L> {
    fn keywords(&self) -> &[Keyword] {
        &self.keywords
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.parse_block_end(|parser| {
            let Some(node) = parser
                .stream
                .blocks
                .top_mut_as::<PendingLoop<L>>()
                .map(|block| block.node)
            else {
                return parser.raise_unmatched_end(L::END.as_str(), L::NAME);
            };
            let end = parser.prev().span();

            parser.complete_top_block(&end);
            parser.stream.scopes.exit_loop();
            parser.stream.blocks.push_statement(node);

            Ok(())
        })
    }
}
