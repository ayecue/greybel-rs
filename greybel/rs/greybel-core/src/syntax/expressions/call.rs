use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeKind, NodeList};
use crate::error::ParseResult;
use crate::lexer::Punct;
use crate::parser::{ExprContext, InfixRule, Parser, Precedence, Trigger, groups};
use crate::position::Span;

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Call {
    #[child]
    pub base: NodeId,
    #[child]
    pub arguments: NodeList,
    pub optional: bool,
}

impl AstNode for Call {
    fn type_name(&self) -> &'static str {
        "CallExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][{}{}({})]",
            ctx.location(),
            ctx.child(self.base),
            if self.optional { "?." } else { "" },
            ctx.list(self.arguments)
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CallRule;

impl InfixRule for CallRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Punct(Punct::LParen)]
    }

    fn precedence(&self) -> Precedence {
        Precedence::Call
    }

    fn applies(&self, parser: &Parser<'_>, left: NodeId, context: ExprContext) -> bool {
        let callable = match &parser.stream.nodes.get(left).kind {
            NodeKind::Identifier(_) | NodeKind::Member(_) => true,
            NodeKind::Index(index) => index.is_statement_start,
            _ => false,
        };

        callable && (!context.as_lval || !parser.token().has_whitespace_before())
    }

    fn parse(
        &self,
        parser: &mut Parser<'_>,
        left: NodeId,
        start: Span,
        _context: ExprContext,
    ) -> ParseResult<NodeId> {
        let arguments = Call::parse_arguments(parser)?;

        Ok(parser.alloc(
            Call {
                base: left,
                arguments,
                optional: false,
            },
            parser.span_from(&start),
        ))
    }
}

impl Call {
    pub fn parse_arguments(parser: &mut Parser<'_>) -> ParseResult<NodeList> {
        if !parser.token().is_punct(Punct::LParen) {
            return Ok(NodeList::EMPTY);
        }

        parser.advance()?;
        if parser.token().is_punct(Punct::RParen) {
            parser.advance()?;

            return Ok(NodeList::EMPTY);
        }

        let mut arguments = parser.stream.nodes.list_buffer();

        loop {
            parser.consume_noop()?;
            if parser.grammar().trailing_call_comma && parser.token().is_punct(Punct::RParen) {
                parser.advance()?;
                break;
            }

            let argument = parser.parse_expr(None)?;

            arguments.push(argument);
            if parser.grammar().relaxed_separators {
                parser.consume_noop()?;
            }

            let from = parser.stream.nodes.get(argument).span.start;
            let has_separator = parser.require_any(groups::CALL_ARGUMENTS_END, Some(from))?;

            if has_separator && parser.prev().is_punct(Punct::RParen)
                || !has_separator && parser.token().is_eof()
            {
                break;
            }
        }

        Ok(parser.stream.nodes.finish_list(arguments))
    }
}
