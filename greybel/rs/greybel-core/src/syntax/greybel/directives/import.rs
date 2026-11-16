use greybel_symbols::contextual_keywords::FROM;

use super::{emit_directive, parse_path};
use crate::ast::{NodeFieldContainer, NodeList, TextSpan};
use crate::diagnostics::GreybelDiagnostic;
use crate::dialect::SyntaxFeatures;
use crate::lexer::Punct;
use crate::parser::KeywordStatement;
use crate::position::Span;
use crate::syntax::Identifier;
use crate::syntax::greybel::{DestructuringAssignment, DestructuringExpansion};
use crate::{AstNode, DumpContext, IdentifierKind, Keyword, NodeId, ParseResult, Parser};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Import {
    #[child]
    pub name: NodeId,
    pub path: TextSpan,
    /// `#import {parse, format as fmt} from "path"`: the assignments from the module (`name`, a `__m`
    /// temporary) to the names, and the source of the names for dev mode.
    #[child]
    pub bindings: NodeList,
    /// The source of `{parse, format as fmt}`.
    pub target: Option<Span>,
}

impl Import {
    /// Name of the temporary that stores the module of a `#import {...}`.
    pub const MODULE: &'static str = "__m";
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
        let target_start = parser.token().span();
        let target = if parser
            .grammar()
            .allows(SyntaxFeatures::DESTRUCTURING_ASSIGNMENTS)
            && parser.token().is_punct(Punct::LBrace)
        {
            Some(DestructuringAssignment::parse_target(parser)?)
        } else {
            None
        };
        let target_span = target
            .as_ref()
            .map(|_| target_start.to(&parser.prev().span()));
        let name = match target {
            Some(_) => {
                let name = parser.stream.nodes.insert_atom(Import::MODULE);
                let identifier = parser.alloc(
                    Identifier {
                        name,
                        kind: IdentifierKind::Variable,
                    },
                    Span::empty(start.start),
                );

                parser.scope_data_mut().namespaces.push(identifier);

                identifier
            }
            None => Identifier::parse(parser, IdentifierKind::Variable)?,
        };

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
        let bindings = match &target {
            Some(target) => {
                let module = parser.stream.nodes.clone_subtree(name);
                let statements =
                    DestructuringExpansion::expand(parser, target, module, start.start)?;

                parser.stream.nodes.push_list(&statements)
            }
            None => NodeList::EMPTY,
        };
        let item = parser.alloc(
            Import {
                name,
                path,
                bindings,
                target: target_span,
            },
            parser.span_from(&start),
        );

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
