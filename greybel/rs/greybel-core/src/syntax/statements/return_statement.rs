use super::shortcut::is_at_clause_end;
use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId};
use crate::error::ParseResult;
use crate::lexer::Keyword;
use crate::parser::{KeywordStatement, Parser, groups};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Return {
    #[child]
    pub argument: Option<NodeId>,
}

impl AstNode for Return {
    fn type_name(&self) -> &'static str {
        "ReturnStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][{}]",
            ctx.location(),
            ctx.optional(self.argument)
        )
    }
}

impl Return {
    pub fn parse(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
        let scope = parser.stream.scopes.current();
        let start = parser.prev().span();
        let node = parser.alloc(Return { argument: None }, start.open());

        let ends = parser.is_at_any(groups::RETURN_END)
            || (parser.grammar().return_before_else && is_at_clause_end(parser));
        let argument = if ends {
            None
        } else {
            Some(parser.parse_expr(Some(node))?)
        };
        let end = parser.prev().span();

        parser.stream.nodes.close(node, &end);
        parser.stream.nodes.set_kind(node, Return { argument });

        if let Some(scope) = scope.and_then(|s| parser.stream.scopes.pending_mut(s)) {
            scope.returns.push(node);
        }

        Ok(node)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReturnRule;

impl KeywordStatement for ReturnRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::Return]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        let depth = parser.stream.blocks.depth();

        parser.advance()?;

        let item = Return::parse(parser)?;
        let closed = parser.stream.nodes.get(item).span.is_closed();

        parser.emit_statement(depth, item, closed);

        Ok(())
    }
}
