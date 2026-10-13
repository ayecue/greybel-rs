use super::{emit_directive, parse_path};
use crate::ast::{NodeFieldContainer, TextSpan};
use crate::parser::{ExprContext, KeywordStatement, PrefixRule, Trigger};
use crate::{AstNode, DumpContext, Keyword, NodeId, ParseResult, Parser};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Inject {
    pub path: TextSpan,
}

impl AstNode for Inject {
    fn type_name(&self) -> &'static str {
        "FeatureInjectExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][path = {}]",
            ctx.location(),
            ctx.read(self.path)
        )
    }
}

fn parse_inject(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
    parser.advance()?;

    let start = parser.prev().span();
    let path = parse_path(parser)?;

    parser.advance()?;

    let item = parser.alloc(Inject { path }, parser.span_from(&start));

    parser
        .stream
        .chunk_data_mut()
        .current_mut()
        .injects
        .push(item);

    Ok(item)
}

#[derive(Debug, Clone, Copy)]
pub struct InjectStatementRule;

impl KeywordStatement for InjectStatementRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::Inject]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        let item = parse_inject(parser)?;

        emit_directive(parser, item)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InjectExpressionRule;

impl PrefixRule for InjectExpressionRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Keyword(Keyword::Inject)]
    }

    fn parse(&self, parser: &mut Parser<'_>, _context: ExprContext) -> ParseResult<NodeId> {
        parse_inject(parser)
    }
}
