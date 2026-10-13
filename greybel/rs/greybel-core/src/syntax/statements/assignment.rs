use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId};
use crate::error::ParseResult;
use crate::lexer::Punct;
use crate::parser::Parser;
use crate::position::Span;
use crate::syntax::{Binary, BinaryOp, Invalid, Parenthesis};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Assignment {
    #[child]
    pub variable: NodeId,
    #[child]
    pub init: NodeId,
    pub shorthand: bool,
}

impl AstNode for Assignment {
    fn type_name(&self) -> &'static str {
        "AssignmentStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][{} = {}]",
            ctx.location(),
            ctx.child(self.variable),
            ctx.child(self.init)
        )
    }
}

pub(crate) fn shorthand_operator(punct: Punct) -> Option<BinaryOp> {
    Some(match punct {
        Punct::PlusAssign => BinaryOp::Add,
        Punct::MinusAssign => BinaryOp::Subtract,
        Punct::AsteriskAssign => BinaryOp::Multiply,
        Punct::SlashAssign => BinaryOp::Divide,
        Punct::CaretAssign => BinaryOp::Power,
        Punct::PercentAssign => BinaryOp::Modulo,
        _ => return None,
    })
}

impl Assignment {
    pub fn parse(parser: &mut Parser<'_>, target: NodeId, start: Span) -> ParseResult<NodeId> {
        let scope = parser.stream.scopes.current();

        parser.advance()?;

        let assignment = parser.alloc(Invalid, start.open());
        let previous = parser.stream.scopes.replace_assignment(Some(assignment));
        let init = parser.parse_expr(Some(assignment))?;

        Self::finish(parser, assignment, target, init, false, (scope, previous));

        Ok(assignment)
    }

    pub fn parse_shorthand(
        parser: &mut Parser<'_>,
        target: NodeId,
        operator: BinaryOp,
    ) -> ParseResult<NodeId> {
        let scope = parser.stream.scopes.current();
        let start = parser.stream.nodes.get(target).span;

        parser.advance()?;

        let assignment = parser.alloc(Invalid, start.open());
        let previous = parser.stream.scopes.replace_assignment(Some(assignment));
        let binary_start = parser.token().span();
        let value = parser.parse_expr(Some(assignment))?;
        let value_span = parser.stream.nodes.get(value).span;
        let right = parser
            .stream
            .nodes
            .alloc(Parenthesis { expression: value }, value_span, None);
        let left = parser.stream.nodes.clone_subtree(target);
        let binary_span = parser.span_from(&binary_start);
        let init = parser.stream.nodes.alloc(
            Binary {
                operator,
                left,
                right,
            },
            binary_span,
            scope,
        );

        Self::finish(parser, assignment, target, init, true, (scope, previous));

        Ok(assignment)
    }

    fn finish(
        parser: &mut Parser<'_>,
        assignment: NodeId,
        variable: NodeId,
        init: NodeId,
        shorthand: bool,
        (scope, previous): (Option<NodeId>, Option<NodeId>),
    ) {
        let end = parser.prev().span();

        parser.stream.nodes.close(assignment, &end);
        parser.stream.nodes.set_kind(
            assignment,
            Assignment {
                variable,
                init,
                shorthand,
            },
        );
        parser.stream.scopes.replace_assignment(previous);
        if let Some(scope) = scope.and_then(|s| parser.stream.scopes.open_mut(s)) {
            scope.definitions.push(assignment);
        }
    }
}
