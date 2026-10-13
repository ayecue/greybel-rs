use greybel_symbols::contextual_keywords::{NO_EMIT, NO_EVAL, OVERRIDE};

use crate::ast::{NodeFieldContainer, TextSpan};
use crate::diagnostics::GreyScriptDiagnostic;
use crate::lexer::offset;
use crate::parser::{Expect, KeywordStatement};
use crate::{AstNode, DumpContext, Keyword, ParseResult, Parser, Punct, Span};

#[derive(Debug, Clone, PartialEq, Eq, NodeFieldContainer)]
pub struct ImportCode {
    pub original_directory: TextSpan,
    pub directory: TextSpan,
    pub eval: bool,
    pub emit: bool,
}

impl AstNode for ImportCode {
    fn type_name(&self) -> &'static str {
        "ImportCodeExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!(
            "ImportCode[{}][{};{};{}]",
            ctx.location(),
            ctx.read(self.directory),
            self.eval,
            self.emit
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportCodeMeta {
    pub eval: bool,
    pub emit: bool,
    pub directory: Option<TextSpan>,
}

impl ImportCodeMeta {
    const ASSIGN: char = '=';
    const SEPARATOR: char = ';';

    #[must_use]
    pub fn parse(meta: &str, span: Span) -> Self {
        let mut result = ImportCodeMeta {
            eval: true,
            emit: true,
            directory: None,
        };
        let mut start = span.start;

        for segment in meta.split(Self::SEPARATOR) {
            let entry = segment.trim();
            let mut parts = entry.split(Self::ASSIGN);
            let key = parts.next().unwrap_or_default();
            let value = parts.next();

            match (key, value) {
                (NO_EVAL, _) => result.eval = false,
                (NO_EMIT, _) => result.emit = false,
                (OVERRIDE, Some(directory)) => {
                    let prefix_len = segment.len() - segment.trim_start().len();
                    let begin = start + offset(prefix_len + key.len() + Self::ASSIGN.len_utf8());
                    let end = begin + offset(directory.len());

                    result.directory = Some(TextSpan::new(Span::new(begin, end)));
                }
                _ => {}
            }

            start += offset(segment.len() + Self::SEPARATOR.len_utf8());
        }

        result
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ImportCodeRule;

impl KeywordStatement for ImportCodeRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::ImportCode]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.advance()?;

        let start = parser.prev().span();

        if !parser.consume(Expect::Punct(Punct::LParen))? {
            return invalid(
                parser,
                &start,
                GreyScriptDiagnostic::ImportCodeMissingOpenParen,
            );
        }

        let Some(string) = parser.token().as_string() else {
            return invalid(
                parser,
                &start,
                GreyScriptDiagnostic::ImportCodeArgumentNotString,
            );
        };
        let mut directory = string.text();

        parser.advance()?;

        if parser.consume(Expect::SliceOperator)? {
            let Some(string) = parser.token().as_string() else {
                return invalid(
                    parser,
                    &start,
                    GreyScriptDiagnostic::ImportCodeArgumentNotString,
                );
            };

            directory = string.text();
            parser.advance()?;
        }

        if !parser.consume(Expect::Punct(Punct::RParen))? {
            return invalid(
                parser,
                &start,
                GreyScriptDiagnostic::ImportCodeMissingCloseParen,
            );
        }

        let end = parser.prev().span();

        let meta = parser.token().as_comment().map_or_else(
            || ImportCodeMeta::parse("", Span::empty(0)),
            |comment| ImportCodeMeta::parse(comment.text(parser.source()), comment.text_span()),
        );
        let node = ImportCode {
            directory: meta.directory.unwrap_or(directory),
            original_directory: directory,
            eval: meta.eval,
            emit: meta.emit,
        };
        let item = parser.alloc(node, start.to(&end));

        parser
            .stream
            .chunk_data_mut()
            .current_mut()
            .native_imports
            .push(item);

        let depth = parser.stream.blocks().depth();

        parser.emit_statement(depth, item, true);

        Ok(())
    }
}

fn invalid(
    parser: &mut Parser<'_>,
    start: &Span,
    diagnostic: GreyScriptDiagnostic,
) -> ParseResult<()> {
    let span = start.to(&parser.token().span());

    parser.raise_at(diagnostic, span)?;

    let invalid = parser.parse_invalid_code()?;
    let depth = parser.stream.blocks().depth();

    parser.emit_statement(depth, invalid, true);

    Ok(())
}
