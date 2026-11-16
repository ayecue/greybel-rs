use crate::ast::NodeFieldContainer;
use crate::diagnostics::GreybelDiagnostic;
use crate::dialect::SyntaxFeatures;
use crate::lexer::TokenKind;
use crate::parser::{ExprContext, InfixRule, Precedence, Trigger};
use crate::syntax::{Call, Index, Member};
use crate::{AstNode, DumpContext, NodeId, ParseResult, Parser, Punct, Span};

/// `condition ? if_true : if_false`.
#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Ternary {
    #[child]
    pub condition: NodeId,
    #[child]
    pub if_true: NodeId,
    #[child]
    pub if_false: NodeId,
}

impl AstNode for Ternary {
    fn type_name(&self) -> &'static str {
        "TernaryExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][{} ? {} : {}]",
            ctx.location(),
            ctx.child(self.condition),
            ctx.child(self.if_true),
            ctx.child(self.if_false)
        )
    }
}

/// `left ?? right`: `left` unless it is `null`, then `right`.
#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Coalesce {
    #[child]
    pub left: NodeId,
    #[child]
    pub right: NodeId,
}

impl AstNode for Coalesce {
    fn type_name(&self) -> &'static str {
        "CoalesceExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][{} ?? {}]",
            ctx.location(),
            ctx.child(self.left),
            ctx.child(self.right)
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TernaryRule;

impl InfixRule for TernaryRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Punct(Punct::Question)]
    }

    fn precedence(&self) -> Precedence {
        Precedence::Ternary
    }

    fn applies(&self, parser: &Parser<'_>, _left: NodeId, _context: ExprContext) -> bool {
        parser.grammar().allows(SyntaxFeatures::TERNARY)
    }

    fn parse(
        &self,
        parser: &mut Parser<'_>,
        condition: NodeId,
        start: Span,
        _context: ExprContext,
    ) -> ParseResult<NodeId> {
        parser.advance()?;
        parser.consume_noop()?;

        let if_true = parser.parse_expr(None)?;

        parser.consume_noop()?;

        if parser.token().kind() != TokenKind::SliceOperator {
            let span = start.to(&parser.token().span());

            parser.raise_at(GreybelDiagnostic::MissingTernaryElse, span)?;

            return Ok(if_true);
        }

        parser.advance()?;
        parser.consume_noop()?;

        // Right-associative: `a ? b : c ? d : e` is `a ? b : (c ? d : e)`.
        let min = parser.rank(Precedence::Ternary);
        let if_false = parser.parse_expr_precedence(min, ExprContext::default())?;

        parser.stream.expands_values = true;

        Ok(parser.alloc(
            Ternary {
                condition,
                if_true,
                if_false,
            },
            parser.span_from(&start),
        ))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CoalesceRule;

impl InfixRule for CoalesceRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Punct(Punct::Coalesce)]
    }

    fn precedence(&self) -> Precedence {
        Precedence::Coalesce
    }

    fn applies(&self, parser: &Parser<'_>, _left: NodeId, _context: ExprContext) -> bool {
        parser.grammar().allows(SyntaxFeatures::COALESCE)
    }

    fn parse(
        &self,
        parser: &mut Parser<'_>,
        left: NodeId,
        start: Span,
        _context: ExprContext,
    ) -> ParseResult<NodeId> {
        let right = parser.parse_right_operand(Precedence::Coalesce)?;

        parser.stream.expands_values = true;

        Ok(parser.alloc(Coalesce { left, right }, parser.span_from(&start)))
    }
}

/// `base?.name`, `base?.[index]` and `base?.(arguments)`.
#[derive(Debug, Clone, Copy)]
pub struct OptionalChainRule;

impl InfixRule for OptionalChainRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Punct(Punct::OptionalDot)]
    }

    fn precedence(&self) -> Precedence {
        Precedence::Call
    }

    fn applies(&self, parser: &Parser<'_>, _left: NodeId, _context: ExprContext) -> bool {
        parser.grammar().allows(SyntaxFeatures::OPTIONAL_CHAINS)
    }

    fn parse(
        &self,
        parser: &mut Parser<'_>,
        base: NodeId,
        start: Span,
        context: ExprContext,
    ) -> ParseResult<NodeId> {
        parser.advance()?;
        parser.stream.expands_values = true;

        if parser.token().is_punct(Punct::LBracket) && !parser.token().has_whitespace_before() {
            return Index::parse_brackets(parser, base, start, context, true);
        }

        if parser.token().is_punct(Punct::LParen) && !parser.token().has_whitespace_before() {
            let arguments = Call::parse_arguments(parser)?;

            return Ok(parser.alloc(
                Call {
                    base,
                    arguments,
                    optional: true,
                },
                parser.span_from(&start),
            ));
        }

        parser.consume_noop()?;

        Member::parse_name(parser, base, start, true)
    }
}
