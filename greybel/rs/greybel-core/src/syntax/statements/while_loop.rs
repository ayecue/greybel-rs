use super::loop_block::{LoopNode, PendingLoop};
use super::shortcut::parse_shortcut_statement;
use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeList};
use crate::error::ParseResult;
use crate::lexer::Keyword;
use crate::parser::{KeywordStatement, Parser, groups};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct While {
    #[child]
    pub condition: NodeId,
    #[child]
    #[body]
    pub body: NodeList,
}

impl AstNode for While {
    fn type_name(&self) -> &'static str {
        "WhileStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();
        let loc = ctx.location();
        let condition = ctx.child(self.condition);

        if self.body.is_empty() {
            format!("{type_name}[{loc}][{condition}]")
        } else {
            format!(
                "{type_name}[{loc}][{condition}\n{}\n]",
                ctx.block(self.body)
            )
        }
    }
}

impl LoopNode for While {
    const END: Keyword = Keyword::EndWhile;
    const NAME: &'static str = Keyword::While.as_str();
}

#[derive(Debug, Clone, Copy)]
pub struct WhileRule;

impl KeywordStatement for WhileRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::While]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.advance()?;

        let start = parser.prev().span();
        let condition = parser.parse_expr(None)?;

        if parser.grammar().single_line_loops && !parser.is_at_any(groups::BLOCK_END_OF_LINE) {
            let depth = parser.stream.blocks.depth();
            let item = parse_shortcut_statement(parser)?;
            let body = parser.stream.nodes.push_list(&[item]);
            let node = parser.alloc(While { condition, body }, parser.span_from(&start));

            parser.emit_statement(depth, node, true);

            return Ok(());
        }

        let body = NodeList::EMPTY;
        let node = parser.alloc(While { condition, body }, start.open());

        PendingLoop::<While>::open(parser, node);

        Ok(())
    }
}
