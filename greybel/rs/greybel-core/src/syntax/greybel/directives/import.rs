use greybel_symbols::contextual_keywords::FROM;

use super::{emit_directive, parse_path};
use crate::ast::{NodeFieldContainer, TextSpan};
use crate::diagnostics::GreybelDiagnostic;
use crate::parser::KeywordStatement;
use crate::syntax::Identifier;
use crate::{AstNode, DumpContext, IdentifierKind, Keyword, NodeId, ParseResult, Parser};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Import {
    #[child]
    pub name: NodeId,
    pub path: TextSpan,
}

impl AstNode for Import {
    fn type_name(&self) -> &'static str {
        "FeatureImportExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][name = {}, path = {}]",
            ctx.location(),
            ctx.child(self.name),
            ctx.read(self.path)
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ImportRule;

impl KeywordStatement for ImportRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::Import, Keyword::ImportWithComment]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.advance()?;

        let start = parser.prev().span();
        let name = Identifier::parse(parser, IdentifierKind::Variable)?;

        if parser
            .token()
            .as_identifier()
            .map(|token| token.name(parser.source()))
            != Some(FROM)
        {
            let span = start.to(&parser.token().span());

            parser.raise_at(GreybelDiagnostic::MissingFromKeyword, span)?;

            let invalid = parser.parse_invalid_code()?;
            let depth = parser.stream.blocks().depth();

            parser.emit_statement(depth, invalid, true);

            return Ok(());
        }

        parser.advance()?;

        let path = parse_path(parser)?;
        let item = parser.alloc(Import { name, path }, parser.span_from(&start));

        parser
            .stream
            .chunk_data_mut()
            .current_mut()
            .imports
            .push(item);
        parser.scope_data_mut().definitions.push(item);

        emit_directive(parser, item)
    }
}
