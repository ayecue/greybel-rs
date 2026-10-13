use super::{ExprContext, Trigger};
use crate::ast::NodeId;
use crate::error::ParseResult;
use crate::parser::{Parser, Precedence};

pub trait PrefixRule: Sync {
    fn triggers(&self) -> &[Trigger];

    fn precedence(&self) -> Option<Precedence> {
        None
    }

    fn parse(&self, parser: &mut Parser<'_>, context: ExprContext) -> ParseResult<NodeId>;
}
