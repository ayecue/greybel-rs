use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId};
use crate::error::ParseResult;
use crate::lexer::Punct;
use crate::parser::{Expect, ExprContext, Parser, PrefixRule, Trigger};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Parenthesis {
    #[child]
    pub expression: NodeId,
}

impl AstNode for Parenthesis {
    fn type_name(&self) -> &'static str {
        "ParenthesisExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        ctx.child(self.expression)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParenthesisRule;

impl PrefixRule for ParenthesisRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Punct(Punct::LParen)]
    }

    fn parse(&self, parser: &mut Parser<'_>, _context: ExprContext) -> ParseResult<NodeId> {
        let start = parser.token().span();

        parser.advance()?;
        parser.consume_noop()?;

        let expression = parser.parse_expr(None)?;

        parser.require(Expect::Punct(Punct::RParen), Some(start.start))?;

        Ok(parser.alloc(Parenthesis { expression }, parser.span_from(&start)))
    }
}
