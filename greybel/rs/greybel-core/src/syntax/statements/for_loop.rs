use super::loop_block::{LoopNode, PendingLoop};
use super::shortcut::parse_shortcut_statement;
use crate::ast::{
    AstNode, DumpContext, LiteralValue, NodeFieldContainer, NodeId, NodeKind, NodeList, NodeStore,
    UnaryOp,
};
use crate::diagnostics::StatementDiagnostic;
use crate::error::ParseResult;
use crate::lexer::Keyword;
use crate::parser::{Expect, KeywordStatement, Parser, groups};
use crate::syntax::{Identifier, IdentifierKind};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct For {
    #[child]
    pub variable: NodeId,
    #[child]
    pub sequence: NodeId,
    #[child]
    #[body]
    pub body: NodeList,
}

impl AstNode for For {
    fn type_name(&self) -> &'static str {
        "ForGenericStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();
        let loc = ctx.location();
        let head = format!(
            "{} in {}",
            ctx.child(self.variable),
            ctx.child(self.sequence)
        );

        if self.body.is_empty() {
            format!("{type_name}[{loc}][{head}]")
        } else {
            format!("For[{loc}][{head}\n{}\n]", ctx.block(self.body))
        }
    }
}

impl LoopNode for For {
    const END: Keyword = Keyword::EndFor;
    const NAME: &'static str = Keyword::For.as_str();
}

#[derive(Debug, Clone, Copy)]
pub struct ForRule;

impl KeywordStatement for ForRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::For]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.advance()?;

        let scope = parser.stream.scopes.current();
        let start = parser.prev().span();
        let variable = Identifier::parse(parser, IdentifierKind::ForInVariable)?;

        parser.require(Expect::Keyword(Keyword::In), Some(start.start))?;

        let sequence = parser.parse_expr(None)?;

        if is_null(&parser.stream.nodes, sequence) {
            let span = parser.stream.nodes.get(sequence).span;

            parser.raise_at(StatementDiagnostic::ForWithoutSequence, span)?;
        }

        let shortcut =
            parser.grammar().single_line_loops && !parser.is_at_any(groups::BLOCK_END_OF_LINE);
        let depth = parser.stream.blocks.depth();
        let body = if shortcut {
            let item = parse_shortcut_statement(parser)?;

            parser.stream.nodes.push_list(&[item])
        } else {
            NodeList::EMPTY
        };
        let node = parser.alloc(
            For {
                variable,
                sequence,
                body,
            },
            parser.span_from(&start),
        );

        if let Some(scope) = scope.and_then(|s| parser.stream.scopes.open_mut(s)) {
            scope.definitions.push(node);
        }

        if shortcut {
            parser.emit_statement(depth, node, true);
        } else {
            PendingLoop::<For>::open(parser, node);
        }

        Ok(())
    }
}

fn is_null(nodes: &NodeStore, id: crate::ast::NodeId) -> bool {
    match &nodes.get(id).kind {
        NodeKind::Literal(literal) => literal.value == LiteralValue::Nil,
        NodeKind::Parenthesis(parenthesis) => is_null(nodes, parenthesis.expression),
        NodeKind::Unary(unary) if unary.operator == UnaryOp::AddressOf => {
            is_null(nodes, unary.argument)
        }
        _ => false,
    }
}
