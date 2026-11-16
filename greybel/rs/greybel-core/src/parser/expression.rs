use greybel_symbols::contextual_keywords::AS;

use super::rules::{Associativity, ExprContext};
use super::{MAX_NODE_DEPTH, Parser, Precedence};
use crate::ast::{IdentifierKind, NodeId};
use crate::diagnostics::{DepthDiagnostic, ExpectationDiagnostic};
use crate::dialect::SyntaxFeatures;
use crate::error::ParseResult;
use crate::lexer::{Keyword, Punct};
use crate::syntax::{Function, Identifier, Invalid, Literal};

impl Parser<'_> {
    pub fn parse_expr(&mut self, base: Option<NodeId>) -> ParseResult<NodeId> {
        self.parse_expr_with(base, ExprContext::default())
    }

    pub fn parse_expr_with(
        &mut self,
        base: Option<NodeId>,
        context: ExprContext,
    ) -> ParseResult<NodeId> {
        if self.token().is_keyword(Keyword::Function) {
            return Function::parse(self, base);
        }

        if self.grammar().allows(SyntaxFeatures::SHORT_FUNCTIONS)
            && self.token().is_punct(Punct::LParen)
            && Function::is_arrow_at(self.source(), self.token().span().start)
        {
            return Function::parse_arrow(self, base);
        }

        self.parse_expr_precedence(0, context)
    }

    pub fn parse_expr_precedence(&mut self, min: u8, context: ExprContext) -> ParseResult<NodeId> {
        if self.stream.expression_depth + self.stream.blocks.depth() >= MAX_NODE_DEPTH {
            let span = self.token().span();

            self.raise_at(
                DepthDiagnostic::DepthLimit {
                    limit: MAX_NODE_DEPTH,
                },
                span,
            )?;

            return self.parse_invalid_code();
        }

        self.stream.expression_depth += 1;

        let result = self.parse_operators(min, context);

        self.stream.expression_depth -= 1;

        result
    }

    fn parse_operators(&mut self, min: u8, context: ExprContext) -> ParseResult<NodeId> {
        let start = self.token().span();
        let mut left = match self.prefix_rule(min) {
            Some(rule) => rule.parse(self, context)?,
            None => self.parse_atom()?,
        };

        let mut current_level = None;

        while !self.token().is_eof() {
            let operator_start = self.token().span().start;

            if self.stream.refused_operator == Some(operator_start) {
                break;
            }

            if self.is_at_as_type() {
                if self.rank(Precedence::Call) < min {
                    break;
                }

                left = self.parse_as_type(left, start)?;

                continue;
            }

            let Some((rule, rank)) = self.infix_rule() else {
                break;
            };

            if rank < min || !rule.applies(self, left, context) {
                break;
            }

            if current_level == Some(rank) {
                self.stream.refused_operator = Some(operator_start);
                break;
            }

            left = rule.parse(self, left, start, context)?;
            current_level = (rule.associativity() == Associativity::None).then_some(rank);
        }

        Ok(left)
    }

    /// `as` after an expression, with types on: `expression as T`.
    fn is_at_as_type(&self) -> bool {
        self.grammar().allows(SyntaxFeatures::TYPES)
            && self
                .token()
                .as_identifier()
                .is_some_and(|identifier| identifier.name(self.source()) == AS)
    }

    pub fn parse_right_operand(&mut self, level: Precedence) -> ParseResult<NodeId> {
        self.advance()?;
        self.consume_noop()?;

        let min = self.rank(level) + 1;

        self.parse_expr_precedence(min, ExprContext::default())
    }

    fn parse_atom(&mut self) -> ParseResult<NodeId> {
        if self.token().is_literal() {
            return Literal::parse(self);
        }

        if self.token().is_identifier() {
            return Identifier::parse(self, IdentifierKind::Variable);
        }

        let diagnostic = ExpectationDiagnostic::Expression {
            got: self.token().describe(self.source()),
        };
        let span = self.token().span();

        self.raise_at(diagnostic, span)?;

        self.parse_invalid_code()
    }

    pub fn parse_invalid_code(&mut self) -> ParseResult<NodeId> {
        let invalid = self.stream.nodes.alloc(Invalid, self.token().span(), None);

        self.advance()?;

        Ok(invalid)
    }
}
