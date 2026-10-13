use super::{ExprContext, Trigger};
use crate::ast::NodeId;
use crate::error::ParseResult;
use crate::parser::{Parser, Precedence};
use crate::position::Span;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Associativity {
    #[default]
    Left,
    None,
}

pub trait InfixRule: Sync {
    fn triggers(&self) -> &[Trigger];

    fn precedence(&self) -> Precedence;

    fn associativity(&self) -> Associativity {
        Associativity::Left
    }

    fn applies(&self, _parser: &Parser<'_>, _left: NodeId, _context: ExprContext) -> bool {
        true
    }

    fn parse(
        &self,
        parser: &mut Parser<'_>,
        left: NodeId,
        start: Span,
        context: ExprContext,
    ) -> ParseResult<NodeId>;
}
