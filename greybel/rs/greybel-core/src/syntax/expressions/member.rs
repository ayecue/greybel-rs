use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId};
use crate::error::ParseResult;
use crate::lexer::Punct;
use crate::parser::{ExprContext, InfixRule, Parser, Precedence, Trigger};
use crate::position::Span;
use crate::syntax::{Identifier, IdentifierKind};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Member {
    #[child]
    pub base: NodeId,
    #[child]
    pub identifier: NodeId,
}

impl AstNode for Member {
    fn type_name(&self) -> &'static str {
        "MemberExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][{}.{}]",
            ctx.location(),
            ctx.child(self.base),
            ctx.child(self.identifier)
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemberRule;

impl InfixRule for MemberRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Punct(Punct::Dot)]
    }

    fn precedence(&self) -> Precedence {
        Precedence::Call
    }

    fn parse(
        &self,
        parser: &mut Parser<'_>,
        left: NodeId,
        start: Span,
        _context: ExprContext,
    ) -> ParseResult<NodeId> {
        parser.advance()?;
        parser.consume_noop()?;

        let identifier = Identifier::parse(parser, IdentifierKind::Property)?;
        let member = parser.alloc(
            Member {
                base: left,
                identifier,
            },
            parser.span_from(&start),
        );

        parser.scope_data_mut().namespaces.push(member);

        Ok(member)
    }
}
