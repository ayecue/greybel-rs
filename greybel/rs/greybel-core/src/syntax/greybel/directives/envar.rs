use super::emit_directive;
use crate::ast::{NodeFieldContainer, TextSpan};
use crate::parser::{ExprContext, KeywordStatement, PrefixRule, Trigger};
use crate::{AstNode, DumpContext, Keyword, NodeId, ParseResult, Parser};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Envar {
    pub name: TextSpan,
}

impl AstNode for Envar {
    fn type_name(&self) -> &'static str {
        "FeatureEnvarExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][name = {}]",
            ctx.location(),
            ctx.read(self.name)
        )
    }
}

fn parse_envar(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
    parser.advance()?;

    let start = parser.prev().span();
    let name = parser.token().as_string().map_or_else(
        || TextSpan::new(parser.token().span()),
        |string| string.text(),
    );

    parser.advance()?;

    Ok(parser.alloc(Envar { name }, parser.span_from(&start)))
}

#[derive(Debug, Clone, Copy)]
pub struct EnvarStatementRule;

impl KeywordStatement for EnvarStatementRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::Envar]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        let item = parse_envar(parser)?;

        emit_directive(parser, item)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EnvarExpressionRule;

impl PrefixRule for EnvarExpressionRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Keyword(Keyword::Envar)]
    }

    fn parse(&self, parser: &mut Parser<'_>, _context: ExprContext) -> ParseResult<NodeId> {
        parse_envar(parser)
    }
}
