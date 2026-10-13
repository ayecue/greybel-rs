use greybel_symbols::contextual_keywords::TYPE;

use super::{emit_directive, parse_path};
use crate::ast::{NodeFieldContainer, TextSpan};
use crate::parser::KeywordStatement;
use crate::{AstNode, DumpContext, Keyword, ParseResult, Parser};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Include {
    pub path: TextSpan,
    pub type_only: bool,
}

impl AstNode for Include {
    fn type_name(&self) -> &'static str {
        "FeatureIncludeExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();
        let tag = if self.type_only { ", typeOnly" } else { "" };

        format!(
            "{type_name}[{}][path = {}{tag}]",
            ctx.location(),
            ctx.read(self.path)
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IncludeRule;

impl KeywordStatement for IncludeRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::Include, Keyword::IncludeWithComment]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.advance()?;

        let start = parser.prev().span();
        let type_only = parser
            .token()
            .as_identifier()
            .map(|token| token.name(parser.source()))
            == Some(TYPE);

        if type_only {
            parser.advance()?;
        }

        let path = parse_path(parser)?;
        let item = parser.alloc(Include { path, type_only }, parser.span_from(&start));

        parser
            .stream
            .chunk_data_mut()
            .current_mut()
            .includes
            .push(item);

        emit_directive(parser, item)
    }
}
