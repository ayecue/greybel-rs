use super::shortcut::is_at_clause_end;
use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId};
use crate::error::ParseResult;
use crate::lexer::Punct;
use crate::parser::{Parser, groups};
use crate::position::Span;
use crate::syntax::Call;

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct CallStatement {
    #[child]
    pub expression: NodeId,
}

impl AstNode for CallStatement {
    fn type_name(&self) -> &'static str {
        "CallStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!(
            "CallStatment[{}][{}]",
            ctx.location(),
            ctx.child(self.expression)
        )
    }
}

impl CallStatement {
    pub fn parse_command(
        parser: &mut Parser<'_>,
        callee: NodeId,
        start: Span,
    ) -> ParseResult<NodeId> {
        let scope = parser.stream.scopes.current();
        let mut arguments = parser.stream.nodes.list_buffer();

        while !parser.token().is_eof() {
            arguments.push(parser.parse_expr(None)?);

            if parser.is_at_any(groups::BLOCK_END_OF_LINE) || is_at_clause_end(parser) {
                break;
            }

            if parser.token().is_punct(Punct::Comma) {
                parser.advance()?;
                parser.consume_noop()?;
                continue;
            }

            if parser.require_any(groups::COMMAND_ARGUMENTS_END, Some(start.start))?
                && (parser.prev().is_eol() || parser.prev().is_eof())
            {
                break;
            }
        }

        let span = parser.span_from(&start);
        let arguments = parser.stream.nodes.finish_list(arguments);
        let expression = if arguments.is_empty() {
            callee
        } else {
            parser.stream.nodes.alloc(
                Call {
                    base: callee,
                    arguments,
                },
                span,
                scope,
            )
        };

        Ok(parser
            .stream
            .nodes
            .alloc(CallStatement { expression }, span, scope))
    }
}
