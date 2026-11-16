use std::borrow::Cow;

use greybel_symbols::delimiters::{ESCAPED_QUOTE, QUOTE};

use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeList};
use crate::diagnostics::GreybelDiagnostic;
use crate::error::ParseResult;
use crate::lexer::token::{ESCAPED_CLOSE_BRACE, ESCAPED_OPEN_BRACE};
use crate::lexer::{AnyToken, Punct, TEMPLATE_START, TemplateScan};
use crate::parser::Parser;
use crate::position::Span;

/// `#"Hello {name}!"`: a string of text pieces with an expression between each two. Builds write it as
/// `"Hello " + (name) + "!"`.
#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Template {
    /// The raw text before each expression and after the last one, one more than `expressions`.
    pub texts: Vec<Span>,
    #[child]
    pub expressions: NodeList,
}

impl AstNode for Template {
    fn type_name(&self) -> &'static str {
        "TemplateString"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!(
            "TemplateString[{}][{}]",
            ctx.location(),
            ctx.list(self.expressions)
        )
    }
}

impl Template {
    /// The value of a raw text piece: `""`, `{{` and `}}` stand for `"`, `{` and `}`.
    #[must_use]
    pub fn text(source: &str, span: Span) -> Cow<'_, str> {
        let raw = span.bytes().map_or("", |bytes| &source[bytes]);

        if !raw.contains(ESCAPED_QUOTE)
            && !raw.contains(ESCAPED_OPEN_BRACE)
            && !raw.contains(ESCAPED_CLOSE_BRACE)
        {
            return Cow::Borrowed(raw);
        }

        Cow::Owned(
            raw.replace(ESCAPED_QUOTE, QUOTE)
                .replace(ESCAPED_OPEN_BRACE, "{")
                .replace(ESCAPED_CLOSE_BRACE, "}"),
        )
    }

    /// Parses the template at the current token: each placeholder's expression is parsed from the source tokens
    /// between its braces.
    pub(crate) fn parse(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
        let token = *parser.token();
        let span = token.span();
        let shape = TemplateScan::new(parser.source(), parser.grammar().multiline_strings)
            .template(span.start as usize)
            .expect("the lexer only produces closed templates");
        let mut texts = Vec::with_capacity(shape.placeholders.len() + 1);
        let mut expressions = parser.stream.nodes.list_buffer();
        let mut text_start = span.start + u32::try_from(TEMPLATE_START.len()).unwrap_or(2);

        for placeholder in &shape.placeholders {
            texts.push(Span::new(text_start, placeholder.start - 1));
            expressions.push(Self::parse_placeholder(parser, *placeholder)?);
            text_start = placeholder.end + 1;
        }

        texts.push(Span::new(text_start, span.end - 1));

        let expressions = parser.stream.nodes.finish_list(expressions);

        parser.stream.tokens.cursor.set(span.end as usize);
        parser.stream.tokens.lookahead.push(token);
        parser.advance()?;

        Ok(parser.alloc(Template { texts, expressions }, span))
    }

    fn parse_placeholder(parser: &mut Parser<'_>, placeholder: Span) -> ParseResult<NodeId> {
        parser.stream.tokens.cursor.set(placeholder.start as usize);
        parser.advance()?;

        if parser.token().span().start >= placeholder.end {
            let empty = Span::new(placeholder.start, placeholder.end);

            parser.raise_at(GreybelDiagnostic::EmptyTemplatePlaceholder, empty)?;

            return Ok(parser.alloc(crate::syntax::Invalid, empty));
        }

        let expression = parser.parse_expr(None)?;
        let token: AnyToken = *parser.token();

        if !(token.is_punct(Punct::RBrace) && token.span().start == placeholder.end) {
            parser.raise_at(GreybelDiagnostic::UnclosedTemplatePlaceholder, token.span())?;
        }

        Ok(expression)
    }
}
