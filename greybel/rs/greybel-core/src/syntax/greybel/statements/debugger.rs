use crate::ast::NodeFieldContainer;
use crate::parser::{Expect, KeywordStatement};
use crate::{AstNode, DumpContext, Keyword, ParseResult, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq, NodeFieldContainer)]
pub struct Debugger;

impl AstNode for Debugger {
    fn type_name(&self) -> &'static str {
        "FeatureDebuggerExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!("{type_name}[{}][]", ctx.location())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DebuggerRule;

impl KeywordStatement for DebuggerRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::Debugger]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.advance()?;

        let item = parser.alloc(Debugger, parser.prev().span());

        parser
            .stream
            .chunk_data_mut()
            .current_mut()
            .debuggers
            .push(item);

        let depth = parser.stream.blocks().depth();

        parser.emit_statement(depth, item, true);
        parser.consume(Expect::Eol)?;

        Ok(())
    }
}
