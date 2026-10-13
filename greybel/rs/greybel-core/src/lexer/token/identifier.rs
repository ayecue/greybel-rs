use std::borrow::Cow;

use greybel_symbols::literals::NULL;

use super::{BooleanLiteral, NilLiteral, PendingToken, Token, TokenKind};
use crate::ast::atom_hash;
use crate::lexer::{LexResult, Lexer, TokenStream, identifier_end};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identifier {
    pub(super) span: Span,
    pub(super) after_space: bool,
    pub(super) hash: u32,
}

impl Identifier {
    #[must_use]
    pub fn name<'s>(&self, source: &'s str) -> &'s str {
        self.span.bytes().map_or("", |bytes| &source[bytes])
    }

    #[must_use]
    pub fn hash(&self) -> u32 {
        self.hash
    }
}

impl Token for Identifier {
    fn kind(&self) -> TokenKind {
        TokenKind::Identifier
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        self.after_space
    }

    fn value<'s>(&self, source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed(self.name(source))
    }

    fn describe(&self, source: &str) -> String {
        format!("Identifier({})", self.name(source))
    }

    fn as_identifier(&self) -> Option<&Identifier> {
        Some(self)
    }
}

impl<'src> Lexer<'src> {
    pub fn scan_identifier_or_keyword(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
    ) -> LexResult {
        let from = stream.cursor.index() + 1;

        stream
            .cursor
            .set(identifier_end(self.source.as_bytes(), from));

        let word = token_start.text(self, stream);

        if let Some(keyword) = self.keyword(word) {
            if let Some(start) = self.compound_start(keyword) {
                return self.scan_compound_keyword(stream, start, token_start);
            }

            return Ok(self.finish_keyword(stream, keyword, token_start));
        }

        let token = if let Some(value) = BooleanLiteral::parse(word) {
            token_start.finish(stream, |span, after_space| BooleanLiteral {
                span,
                after_space,
                value,
            })
        } else if word == NULL {
            token_start.finish(stream, |span, after_space| NilLiteral { span, after_space })
        } else {
            token_start.finish(stream, |span, after_space| Identifier {
                span,
                after_space,
                hash: atom_hash(word),
            })
        };

        Ok(token)
    }
}
