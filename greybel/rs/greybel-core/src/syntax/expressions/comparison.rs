use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeList};
use crate::error::ParseResult;
use crate::lexer::{AnyToken, Punct};
use crate::parser::{ExprContext, InfixRule, Parser, Precedence, Trigger};
use crate::position::Span;
use crate::syntax::{Binary, BinaryOp};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct ComparisonGroup {
    pub operators: Vec<BinaryOp>,
    #[child]
    pub expressions: NodeList,
}

impl AstNode for ComparisonGroup {
    fn type_name(&self) -> &'static str {
        "ComparisonGroupExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();
        let expressions = ctx.ids(self.expressions);
        let mut group = vec![ctx.child(expressions[0])];

        for (op, expr) in self.operators.iter().zip(&expressions[1..]) {
            group.push(op.as_str().to_owned());
            group.push(ctx.child(*expr));
        }

        format!("{type_name}[{}][{}]", ctx.location(), group.join(" "))
    }
}

fn comparison_op(token: &AnyToken) -> Option<BinaryOp> {
    match token.punct()? {
        Punct::Equal => Some(BinaryOp::Equal),
        Punct::NotEqual => Some(BinaryOp::NotEqual),
        Punct::Greater => Some(BinaryOp::Greater),
        Punct::GreaterEqual => Some(BinaryOp::GreaterEqual),
        Punct::Less => Some(BinaryOp::Less),
        Punct::LessEqual => Some(BinaryOp::LessEqual),
        _ => None,
    }
}

const COMPARISON_TRIGGERS: &[Trigger] = &[
    Trigger::Punct(Punct::Equal),
    Trigger::Punct(Punct::NotEqual),
    Trigger::Punct(Punct::Greater),
    Trigger::Punct(Punct::GreaterEqual),
    Trigger::Punct(Punct::Less),
    Trigger::Punct(Punct::LessEqual),
];

#[derive(Debug, Clone, Copy)]
pub struct ComparisonRule;

impl InfixRule for ComparisonRule {
    fn triggers(&self) -> &[Trigger] {
        COMPARISON_TRIGGERS
    }

    fn precedence(&self) -> Precedence {
        Precedence::Comparison
    }

    fn parse(
        &self,
        parser: &mut Parser<'_>,
        left: NodeId,
        start: Span,
        _context: ExprContext,
    ) -> ParseResult<NodeId> {
        let operator = comparison_op(parser.token()).expect("the rule triggers on comparisons");
        let right = parser.parse_right_operand(Precedence::Comparison)?;

        if comparison_op(parser.token()).is_none() {
            let span = parser.span_from(&start);

            return Ok(parser.alloc(
                Binary {
                    operator,
                    left,
                    right,
                },
                span,
            ));
        }

        let mut expressions = parser.stream.nodes.list_buffer();

        expressions.extend([left, right]);

        let mut operators = vec![operator];

        while let Some(operator) = comparison_op(parser.token()) {
            operators.push(operator);
            expressions.push(parser.parse_right_operand(Precedence::Comparison)?);
        }

        let span = parser.span_from(&start);
        let expressions = parser.stream.nodes.finish_list(expressions);

        Ok(parser.alloc(
            ComparisonGroup {
                operators,
                expressions,
            },
            span,
        ))
    }
}
