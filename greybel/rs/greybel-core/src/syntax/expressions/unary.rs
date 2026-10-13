use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeKind};
use crate::error::ParseResult;
use crate::lexer::{Keyword, Punct};
use crate::parser::{ExprContext, Parser, Precedence, PrefixRule, Trigger};
use crate::syntax::LiteralValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Not,
    Negate,
    AddressOf,
    New,
}

impl UnaryOp {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            UnaryOp::Not => Keyword::Not.as_str(),
            UnaryOp::Negate => Punct::Minus.as_str(),
            UnaryOp::AddressOf => Punct::At.as_str(),
            UnaryOp::New => Keyword::New.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Unary {
    pub operator: UnaryOp,
    #[child]
    pub argument: NodeId,
}

impl AstNode for Unary {
    fn type_name(&self) -> &'static str {
        match self.operator {
            UnaryOp::Not => "NegationExpression",
            UnaryOp::Negate => "BinaryNegatedExpression",
            UnaryOp::AddressOf | UnaryOp::New => "UnaryExpression",
        }
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!(
            "{}[{}][{} {}]",
            self.type_name(),
            ctx.location(),
            self.operator.as_str(),
            ctx.child(self.argument)
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UnaryRule {
    pub trigger: Trigger,
    pub operator: UnaryOp,
    pub precedence: Precedence,
    pub operand_is_lvalue: bool,
}

impl UnaryRule {
    pub const ADDRESS_OF: UnaryRule = UnaryRule {
        trigger: Trigger::Punct(Punct::At),
        operator: UnaryOp::AddressOf,
        precedence: Precedence::AddressOf,
        operand_is_lvalue: true,
    };
    pub const NEGATE: UnaryRule = UnaryRule {
        trigger: Trigger::Punct(Punct::Minus),
        operator: UnaryOp::Negate,
        precedence: Precedence::UnaryMinus,
        operand_is_lvalue: false,
    };
    pub const NEW: UnaryRule = UnaryRule {
        trigger: Trigger::Keyword(Keyword::New),
        operator: UnaryOp::New,
        precedence: Precedence::New,
        operand_is_lvalue: false,
    };
    pub const NOT: UnaryRule = UnaryRule {
        trigger: Trigger::Keyword(Keyword::Not),
        operator: UnaryOp::Not,
        precedence: Precedence::Not,
        operand_is_lvalue: false,
    };
}

impl PrefixRule for UnaryRule {
    fn triggers(&self) -> &[Trigger] {
        std::slice::from_ref(&self.trigger)
    }

    fn precedence(&self) -> Option<Precedence> {
        Some(self.precedence)
    }

    fn parse(&self, parser: &mut Parser<'_>, context: ExprContext) -> ParseResult<NodeId> {
        let start = parser.token().span();

        parser.advance()?;
        parser.consume_noop()?;

        let operand_context = ExprContext {
            as_lval: self.operand_is_lvalue,
            statement_start: self.operand_is_lvalue && context.statement_start,
        };
        let operand = parser.rank(self.precedence) + 1;
        let argument = parser.parse_expr_precedence(operand, operand_context)?;

        if self.operator == UnaryOp::Negate
            && let NodeKind::Literal(literal) = &mut parser.stream.nodes.get_mut(argument).kind
            && matches!(
                literal.value,
                LiteralValue::Number(_) | LiteralValue::Boolean(_)
            )
        {
            literal.negated = true;

            return Ok(argument);
        }

        let operator = self.operator;

        Ok(parser.alloc(Unary { operator, argument }, parser.span_from(&start)))
    }
}
