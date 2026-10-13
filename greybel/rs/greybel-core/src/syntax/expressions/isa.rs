use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId};
use crate::error::ParseResult;
use crate::lexer::Keyword;
use crate::parser::{Associativity, ExprContext, InfixRule, Parser, Precedence, Trigger};
use crate::position::Span;

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Isa {
    #[child]
    pub left: NodeId,
    #[child]
    pub right: NodeId,
}

impl AstNode for Isa {
    fn type_name(&self) -> &'static str {
        "IsaExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][{} isa {}]",
            ctx.location(),
            ctx.child(self.left),
            ctx.child(self.right)
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IsaRule;

impl InfixRule for IsaRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Keyword(Keyword::Isa)]
    }

    fn precedence(&self) -> Precedence {
        Precedence::Isa
    }

    fn associativity(&self) -> Associativity {
        Associativity::None
    }

    fn parse(
        &self,
        parser: &mut Parser<'_>,
        left: NodeId,
        start: Span,
        _context: ExprContext,
    ) -> ParseResult<NodeId> {
        let right = parser.parse_right_operand(Precedence::Isa)?;

        Ok(parser.alloc(Isa { left, right }, parser.span_from(&start)))
    }
}
