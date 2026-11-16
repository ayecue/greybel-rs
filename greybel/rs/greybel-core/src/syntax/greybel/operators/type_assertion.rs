use crate::ast::{NodeFieldContainer, TextSpan};
use crate::dialect::SyntaxFeatures;
use crate::parser::{ExprContext, InfixRule, Precedence, Trigger};
use crate::{AstNode, DumpContext, NodeId, ParseResult, Parser, Punct, Span};

/// `expression!` or `expression as T`: the expression with a type for the analyzer. Builds write the expression.
#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct TypeAssertion {
    #[child]
    pub expression: NodeId,
    pub target: AssertedType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssertedType {
    /// `expression!`: not `null`.
    NonNull,
    /// `expression as T`, with the span of `T`.
    Type(Span),
}

impl AstNode for TypeAssertion {
    fn type_name(&self) -> &'static str {
        "TypeAssertionExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();
        let target = match self.target {
            AssertedType::NonNull => "!".to_owned(),
            AssertedType::Type(span) => format!(" as {}", ctx.read(TextSpan::new(span))),
        };

        format!(
            "{type_name}[{}][{}{target}]",
            ctx.location(),
            ctx.child(self.expression)
        )
    }
}

/// `expression!`.
#[derive(Debug, Clone, Copy)]
pub struct NonNullRule;

impl InfixRule for NonNullRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Punct(Punct::Bang)]
    }

    fn precedence(&self) -> Precedence {
        Precedence::Call
    }

    fn applies(&self, parser: &Parser<'_>, _left: NodeId, _context: ExprContext) -> bool {
        parser.grammar().allows(SyntaxFeatures::TYPES)
    }

    fn parse(
        &self,
        parser: &mut Parser<'_>,
        left: NodeId,
        start: Span,
        _context: ExprContext,
    ) -> ParseResult<NodeId> {
        parser.advance()?;

        Ok(parser.alloc(
            TypeAssertion {
                expression: left,
                target: AssertedType::NonNull,
            },
            parser.span_from(&start),
        ))
    }
}

impl Parser<'_> {
    /// `as T` after `left`, with types on.
    pub(crate) fn parse_as_type(&mut self, left: NodeId, start: Span) -> ParseResult<NodeId> {
        let type_start = self.token().span().end;
        let Some(ty) = self.parse_type_text(type_start)? else {
            return Ok(left);
        };

        Ok(self.alloc(
            TypeAssertion {
                expression: left,
                target: AssertedType::Type(ty),
            },
            Span::new(start.start, ty.end),
        ))
    }
}
