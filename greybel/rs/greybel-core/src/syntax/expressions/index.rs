use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId};
use crate::error::ParseResult;
use crate::lexer::{Punct, TokenKind};
use crate::parser::{Expect, ExprContext, InfixRule, Parser, Precedence, Trigger};
use crate::position::Span;
use crate::syntax::Empty;

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Index {
    #[child]
    pub base: NodeId,
    #[child]
    pub index: NodeId,
    pub is_statement_start: bool,
    pub optional: bool,
}

impl AstNode for Index {
    fn type_name(&self) -> &'static str {
        "IndexExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}]{}[{}{}[{}]]",
            ctx.location(),
            if self.is_statement_start {
                "[isStatementStart]"
            } else {
                ""
            },
            ctx.child(self.base),
            if self.optional { "?." } else { "" },
            ctx.child(self.index)
        )
    }
}

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Slice {
    #[child]
    pub base: NodeId,
    #[child]
    pub left: NodeId,
    #[child]
    pub right: NodeId,
    pub optional: bool,
}

impl AstNode for Slice {
    fn type_name(&self) -> &'static str {
        "SliceExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][{}{}[{}:{}]]",
            ctx.location(),
            ctx.child(self.base),
            if self.optional { "?." } else { "" },
            ctx.child(self.left),
            ctx.child(self.right)
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IndexRule;

impl InfixRule for IndexRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Punct(Punct::LBracket)]
    }

    fn precedence(&self) -> Precedence {
        Precedence::Call
    }

    fn applies(&self, parser: &Parser<'_>, _left: NodeId, _context: ExprContext) -> bool {
        !parser.token().has_whitespace_before()
    }

    fn parse(
        &self,
        parser: &mut Parser<'_>,
        base: NodeId,
        start: Span,
        context: ExprContext,
    ) -> ParseResult<NodeId> {
        Index::parse_brackets(parser, base, start, context, false)
    }
}

impl Index {
    pub(crate) fn parse_brackets(
        parser: &mut Parser<'_>,
        base: NodeId,
        start: Span,
        context: ExprContext,
        optional: bool,
    ) -> ParseResult<NodeId> {
        parser.advance()?;
        parser.consume_noop()?;

        let left = if parser.token().kind() == TokenKind::SliceOperator {
            empty_at_prev(parser)
        } else {
            let index = parser.parse_expr(None)?;

            if parser.token().kind() != TokenKind::SliceOperator {
                let span = start.to(&parser.token().span());
                let node = parser.alloc(
                    Index {
                        base,
                        index,
                        is_statement_start: context.statement_start,
                        optional,
                    },
                    span,
                );

                parser.require(Expect::Punct(Punct::RBracket), Some(start.start))?;

                return Ok(node);
            }

            index
        };

        parser.advance()?;
        parser.consume_noop()?;

        let right = if parser.token().is_punct(Punct::RBracket) {
            empty_at_prev(parser)
        } else {
            parser.parse_expr(None)?
        };
        let span = start.to(&parser.token().span());
        let node = parser.alloc(
            Slice {
                base,
                left,
                right,
                optional,
            },
            span,
        );

        parser.require(Expect::Punct(Punct::RBracket), Some(start.start))?;

        Ok(node)
    }
}

fn empty_at_prev(parser: &mut Parser<'_>) -> NodeId {
    let span = parser.prev().span();

    parser.alloc(Empty, span)
}
