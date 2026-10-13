use super::assignment::shorthand_operator;
use super::shortcut::is_at_clause_end;
use super::{Assignment, CallStatement};
use crate::ast::{NodeId, NodeKind, NodeStore, UnaryOp};
use crate::diagnostics::{KeywordDiagnostic, StatementDiagnostic};
use crate::error::ParseResult;
use crate::lexer::{Punct, TokenKind};
use crate::parser::{ExprContext, Parser, groups};

pub fn parse_expression_statement(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
    if parser.token().kind() == TokenKind::NilLiteral {
        let span = parser.token().span();

        parser.raise_at(KeywordDiagnostic::MisplacedNull, span)?;

        return parser.parse_invalid_code();
    }

    let start = parser.token().span();
    let context = ExprContext {
        as_lval: true,
        statement_start: true,
    };
    let target = parser.parse_expr_with(None, context)?;

    if parser.is_at_any(groups::ASSIGNMENT_END_OF_EXPR) || is_at_clause_end(parser) {
        return Ok(target);
    }

    if parser.token().is_punct(Punct::Assign) {
        check_target(parser, target)?;

        return Assignment::parse(parser, target, start);
    }

    if let Some(punct) = parser.token().punct()
        && let Some(operator) = shorthand_operator(punct)
    {
        check_target(parser, target)?;

        return Assignment::parse_shorthand(parser, target, operator);
    }

    CallStatement::parse_command(parser, target, start)
}

fn check_target(parser: &mut Parser<'_>, target: NodeId) -> ParseResult<()> {
    if is_assignable(&parser.stream.nodes, target) {
        return Ok(());
    }

    let span = parser.stream.nodes.get(target).span;

    parser.raise_at(StatementDiagnostic::InvalidAssignmentTarget, span)
}

fn is_assignable(nodes: &NodeStore, id: NodeId) -> bool {
    match &nodes.get(id).kind {
        NodeKind::Identifier(_)
        | NodeKind::Member(_)
        | NodeKind::Index(_)
        | NodeKind::Literal(_) => true,
        NodeKind::Unary(unary) if unary.operator == UnaryOp::AddressOf => {
            is_assignable(nodes, unary.argument)
        }
        _ => false,
    }
}
