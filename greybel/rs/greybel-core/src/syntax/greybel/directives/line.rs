use crate::ast::NodeFieldContainer;
use crate::lexer::Keyword;
use crate::parser::{ExprContext, PrefixRule, Trigger};
use crate::{AstNode, DumpContext, NodeId, ParseResult, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq, NodeFieldContainer)]
pub struct Line;

impl AstNode for Line {
    fn type_name(&self) -> &'static str {
        "FeatureLineExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!("{type_name}[{}][]", ctx.location())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LineRule;

impl PrefixRule for LineRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Keyword(Keyword::Line)]
    }

    fn parse(&self, parser: &mut Parser<'_>, _context: ExprContext) -> ParseResult<NodeId> {
        parser.advance()?;

        Ok(parser.alloc(Line, parser.prev().span()))
    }
}
