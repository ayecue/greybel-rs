use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId};
use crate::error::ParseResult;
use crate::lexer::Keyword;
use crate::parser::{ExprContext, InfixRule, Parser, Precedence, Trigger};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalOp {
    And,
    Or,
}

impl LogicalOp {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            LogicalOp::And => Keyword::And.as_str(),
            LogicalOp::Or => Keyword::Or.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Logical {
    pub operator: LogicalOp,
    #[child]
    pub left: NodeId,
    #[child]
    pub right: NodeId,
}

impl AstNode for Logical {
    fn type_name(&self) -> &'static str {
        "LogicalExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][{} {} {}]",
            ctx.location(),
            ctx.child(self.left),
            self.operator.as_str(),
            ctx.child(self.right)
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LogicalRule {
    pub trigger: Trigger,
    pub operator: LogicalOp,
    pub precedence: Precedence,
}

impl LogicalRule {
    pub const AND: LogicalRule = LogicalRule {
        trigger: Trigger::Keyword(Keyword::And),
        operator: LogicalOp::And,
        precedence: Precedence::And,
    };
    pub const OR: LogicalRule = LogicalRule {
        trigger: Trigger::Keyword(Keyword::Or),
        operator: LogicalOp::Or,
        precedence: Precedence::Or,
    };
}

impl InfixRule for LogicalRule {
    fn triggers(&self) -> &[Trigger] {
        std::slice::from_ref(&self.trigger)
    }

    fn precedence(&self) -> Precedence {
        self.precedence
    }

    fn parse(
        &self,
        parser: &mut Parser<'_>,
        left: NodeId,
        start: Span,
        _context: ExprContext,
    ) -> ParseResult<NodeId> {
        let right = parser.parse_right_operand(self.precedence)?;
        let operator = self.operator;

        Ok(parser.alloc(
            Logical {
                operator,
                left,
                right,
            },
            parser.span_from(&start),
        ))
    }
}
