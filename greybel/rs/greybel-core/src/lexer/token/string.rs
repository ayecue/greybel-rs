use std::borrow::Cow;

use greybel_symbols::delimiters::{ESCAPED_QUOTE, QUOTE};

use super::{PendingToken, Token, TokenKind};
use crate::ast::TextSpan;
use crate::diagnostics::LexerDiagnostic;
use crate::error::ParseError;
use crate::lexer::{LexResult, Lexer, TokenStream, is_end_of_line, offset};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringLiteral {
    pub(super) span: Span,
    pub(super) after_space: bool,
    pub(super) escaped: bool,
    pub(super) template: bool,
}

impl StringLiteral {
    #[must_use]
    pub fn unescaped_text<'s>(&self, source: &'s str) -> Cow<'s, str> {
        self.text().read(source)
    }

    #[must_use]
    pub fn raw<'s>(&self, source: &'s str) -> &'s str {
        self.span.bytes().map_or("", |bytes| &source[bytes])
    }

    /// Whether the string is a `#"..."` template with `{expression}` placeholders.
    #[must_use]
    pub fn is_template(&self) -> bool {
        self.template
    }

    #[must_use]
    pub fn text(&self) -> TextSpan {
        let quote = offset(QUOTE.len());
        let start = self.span.start + quote + u32::from(self.template);
        let inner = Span::new(start, self.span.end.saturating_sub(quote).max(start));

        if self.escaped {
            TextSpan::with_escapes(inner)
        } else {
            TextSpan::new(inner)
        }
    }
}

impl Token for StringLiteral {
    fn kind(&self) -> TokenKind {
        TokenKind::StringLiteral
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        self.after_space
    }

    fn value<'s>(&self, source: &'s str) -> Cow<'s, str> {
        self.unescaped_text(source)
    }

    fn describe(&self, source: &str) -> String {
        format!("String({})", self.unescaped_text(source))
    }

    fn display_value<'s>(&self, source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed(self.raw(source))
    }

    fn as_string(&self) -> Option<&StringLiteral> {
        (!self.template).then_some(self)
    }
}

impl<'src> Lexer<'src> {
    pub(crate) fn scan_string(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
    ) -> LexResult {
        stream.cursor.advance(QUOTE.len());

        let multiline = self.grammar().multiline_strings;
        let mut closed = false;
        let mut escaped = false;

        while !self.is_at_end(stream) {
            if self.starts_with(stream, ESCAPED_QUOTE) {
                escaped = true;
                stream.cursor.advance(ESCAPED_QUOTE.len());
            } else if self.starts_with(stream, QUOTE) {
                closed = true;
                break;
            } else if !multiline && is_end_of_line(self.code_at(stream, 0)) {
                break;
            } else {
                stream.cursor.advance(1);
            }
        }

        if !closed {
            let error =
                ParseError::from_lexer(LexerDiagnostic::UnclosedString, token_start.span(stream));

            return self.raise(stream, token_start, error);
        }

        stream.cursor.advance(QUOTE.len());

        Ok(
            token_start.finish(stream, |span, after_space| StringLiteral {
                span,
                after_space,
                escaped,
                template: false,
            }),
        )
    }

    /// `#"..."` with `{expression}` placeholders; `{{`, `}}` and `""` are literal text. Strings, templates and braces
    /// inside a placeholder nest.
    pub(crate) fn scan_template(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
    ) -> LexResult {
        let scan = TemplateScan::new(self.source(), self.grammar().multiline_strings);

        let Some(shape) = scan.template(stream.cursor.index()) else {
            stream.cursor.set(self.source().len());

            let error =
                ParseError::from_lexer(LexerDiagnostic::UnclosedString, token_start.span(stream));

            return self.raise(stream, token_start, error);
        };

        stream.cursor.set(shape.end);

        Ok(
            token_start.finish(stream, |span, after_space| StringLiteral {
                span,
                after_space,
                escaped: true,
                template: true,
            }),
        )
    }
}

/// The placeholders of a template: the spans between each `{` and its `}`, and the offset after the closing quote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateShape {
    pub placeholders: Vec<Span>,
    pub end: usize,
}

/// Scans `#"..."` templates in source text.
#[derive(Debug, Clone, Copy)]
pub struct TemplateScan<'a> {
    source: &'a [u8],
    multiline: bool,
}

impl<'a> TemplateScan<'a> {
    #[must_use]
    pub fn new(source: &'a str, multiline: bool) -> Self {
        Self {
            source: source.as_bytes(),
            multiline,
        }
    }

    /// The shape of the template that starts with `#"` at `start`, or `None` when it isn't closed.
    #[must_use]
    pub fn template(self, start: usize) -> Option<TemplateShape> {
        let mut index = start + TEMPLATE_START.len();
        let mut placeholders = Vec::new();

        while index < self.source.len() {
            let tail = &self.source[index..];

            if tail.starts_with(ESCAPED_QUOTE.as_bytes())
                || tail.starts_with(ESCAPED_OPEN_BRACE.as_bytes())
                || tail.starts_with(ESCAPED_CLOSE_BRACE.as_bytes())
            {
                index += 2;
            } else if tail.starts_with(QUOTE.as_bytes()) {
                return Some(TemplateShape {
                    placeholders,
                    end: index + QUOTE.len(),
                });
            } else if tail[0] == b'{' {
                let end = self.placeholder_end(index + 1)?;

                placeholders.push(Span::new(offset(index + 1), offset(end)));
                index = end + 1;
            } else if !self.multiline && is_end_of_line(tail[0]) {
                return None;
            } else {
                index += 1;
            }
        }

        None
    }

    /// The offset of the `}` that closes a placeholder starting at `start`.
    fn placeholder_end(self, start: usize) -> Option<usize> {
        let mut index = start;
        let mut depth = 0_usize;

        while index < self.source.len() {
            let tail = &self.source[index..];

            if tail.starts_with(TEMPLATE_START.as_bytes()) {
                index = self.template(index)?.end;
            } else if tail.starts_with(QUOTE.as_bytes()) {
                index = self.string_end(index + QUOTE.len())?;
            } else if tail[0] == b'{' {
                depth += 1;
                index += 1;
            } else if tail[0] == b'}' {
                if depth == 0 {
                    return Some(index);
                }

                depth -= 1;
                index += 1;
            } else if !self.multiline && is_end_of_line(tail[0]) {
                return None;
            } else {
                index += 1;
            }
        }

        None
    }

    /// The offset after the closing quote of a plain string whose content starts at `start`.
    fn string_end(self, start: usize) -> Option<usize> {
        let mut index = start;

        while index < self.source.len() {
            let tail = &self.source[index..];

            if tail.starts_with(ESCAPED_QUOTE.as_bytes()) {
                index += ESCAPED_QUOTE.len();
            } else if tail.starts_with(QUOTE.as_bytes()) {
                return Some(index + QUOTE.len());
            } else if !self.multiline && is_end_of_line(tail[0]) {
                return None;
            } else {
                index += 1;
            }
        }

        None
    }
}

/// The start of a string template.
pub const TEMPLATE_START: &str = "#\"";
pub const ESCAPED_OPEN_BRACE: &str = "{{";
pub const ESCAPED_CLOSE_BRACE: &str = "}}";
