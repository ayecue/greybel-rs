use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId};
use crate::error::ParseResult;
use crate::lexer::Punct;
use crate::parser::{ExprContext, InfixRule, Parser, Precedence, Trigger};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
}

impl BinaryOp {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BinaryOp::Add => Punct::Plus.as_str(),
            BinaryOp::Subtract => Punct::Minus.as_str(),
            BinaryOp::Multiply => Punct::Asterisk.as_str(),
            BinaryOp::Divide => Punct::Slash.as_str(),
            BinaryOp::Modulo => Punct::Percent.as_str(),
            BinaryOp::Power => Punct::Caret.as_str(),
            BinaryOp::Equal => Punct::Equal.as_str(),
            BinaryOp::NotEqual => Punct::NotEqual.as_str(),
            BinaryOp::Less => Punct::Less.as_str(),
            BinaryOp::Greater => Punct::Greater.as_str(),
            BinaryOp::LessEqual => Punct::LessEqual.as_str(),
            BinaryOp::GreaterEqual => Punct::GreaterEqual.as_str(),
        }
    }

    #[must_use]
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::Less
                | BinaryOp::Greater
                | BinaryOp::LessEqual
                | BinaryOp::GreaterEqual
        )
    }
}

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Binary {
    pub operator: BinaryOp,
    #[child]
    pub left: NodeId,
    #[child]
    pub right: NodeId,
}

impl AstNode for Binary {
    fn type_name(&self) -> &'static str {
        "BinaryExpression"
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
pub struct BinaryRule {
    pub trigger: Trigger,
    pub operator: BinaryOp,
    pub precedence: Precedence,
}

impl BinaryRule {
    pub const ADD: BinaryRule = BinaryRule {
        trigger: Trigger::Punct(Punct::Plus),
        operator: BinaryOp::Add,
        precedence: Precedence::AddSub,
    };
    pub const DIVIDE: BinaryRule = BinaryRule {
        trigger: Trigger::Punct(Punct::Slash),
        operator: BinaryOp::Divide,
        precedence: Precedence::MulDiv,
    };
    pub const MODULO: BinaryRule = BinaryRule {
        trigger: Trigger::Punct(Punct::Percent),
        operator: BinaryOp::Modulo,
        precedence: Precedence::MulDiv,
    };
    pub const MULTIPLY: BinaryRule = BinaryRule {
        trigger: Trigger::Punct(Punct::Asterisk),
        operator: BinaryOp::Multiply,
        precedence: Precedence::MulDiv,
    };
    pub const POWER: BinaryRule = BinaryRule {
        trigger: Trigger::Punct(Punct::Caret),
        operator: BinaryOp::Power,
        precedence: Precedence::Power,
    };
    pub const SUBTRACT: BinaryRule = BinaryRule {
        trigger: Trigger::Punct(Punct::Minus),
        operator: BinaryOp::Subtract,
        precedence: Precedence::AddSub,
    };
}

impl InfixRule for BinaryRule {
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
            Binary {
                operator,
                left,
                right,
            },
            parser.span_from(&start),
        ))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SubtractRule;

impl InfixRule for SubtractRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Punct(Punct::Minus)]
    }

    fn precedence(&self) -> Precedence {
        Precedence::AddSub
    }

    fn applies(&self, parser: &Parser<'_>, _left: NodeId, context: ExprContext) -> bool {
        !context.statement_start
            || !parser.token().has_whitespace_before()
            || parser.lexer.is_at_whitespace(&parser.stream.tokens)
    }

    fn parse(
        &self,
        parser: &mut Parser<'_>,
        left: NodeId,
        start: Span,
        context: ExprContext,
    ) -> ParseResult<NodeId> {
        InfixRule::parse(&BinaryRule::SUBTRACT, parser, left, start, context)
    }
}
