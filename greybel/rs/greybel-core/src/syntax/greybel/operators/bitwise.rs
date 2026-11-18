use crate::ast::NodeFieldContainer;
use crate::parser::{ExprContext, InfixRule, Precedence, Trigger};
use crate::{AstNode, DumpContext, NodeId, ParseResult, Parser, Punct, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BitwiseOp {
    And,
    Or,
    ShiftLeft,
    ShiftRight,
    UnsignedShiftRight,
}

impl BitwiseOp {
    #[must_use]
    pub fn punct(self) -> Punct {
        match self {
            BitwiseOp::And => Punct::BitwiseAnd,
            BitwiseOp::Or => Punct::BitwiseOr,
            BitwiseOp::ShiftLeft => Punct::ShiftLeft,
            BitwiseOp::ShiftRight => Punct::ShiftRight,
            BitwiseOp::UnsignedShiftRight => Punct::UnsignedShiftRight,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        self.punct().as_str()
    }
}

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Bitwise {
    pub operator: BitwiseOp,
    #[child]
    pub left: NodeId,
    #[child]
    pub right: NodeId,
}

impl AstNode for Bitwise {
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
pub struct BitwiseRule {
    pub trigger: Trigger,
    pub operator: BitwiseOp,
    pub precedence: Precedence,
}

impl BitwiseRule {
    pub const AND: BitwiseRule = BitwiseRule {
        trigger: Trigger::Punct(Punct::BitwiseAnd),
        operator: BitwiseOp::And,
        precedence: Precedence::BitwiseAnd,
    };
    pub const OR: BitwiseRule = BitwiseRule {
        trigger: Trigger::Punct(Punct::BitwiseOr),
        operator: BitwiseOp::Or,
        precedence: Precedence::BitwiseOr,
    };
    pub const SHIFT_LEFT: BitwiseRule = BitwiseRule {
        trigger: Trigger::Punct(Punct::ShiftLeft),
        operator: BitwiseOp::ShiftLeft,
        precedence: Precedence::Shift,
    };
    pub const SHIFT_RIGHT: BitwiseRule = BitwiseRule {
        trigger: Trigger::Punct(Punct::ShiftRight),
        operator: BitwiseOp::ShiftRight,
        precedence: Precedence::Shift,
    };
    pub const UNSIGNED_SHIFT_RIGHT: BitwiseRule = BitwiseRule {
        trigger: Trigger::Punct(Punct::UnsignedShiftRight),
        operator: BitwiseOp::UnsignedShiftRight,
        precedence: Precedence::Shift,
    };
}

impl InfixRule for BitwiseRule {
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
            Bitwise {
                operator,
                left,
                right,
            },
            parser.span_from(&start),
        ))
    }
}
