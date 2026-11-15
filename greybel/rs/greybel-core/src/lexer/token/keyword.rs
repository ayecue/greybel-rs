use std::borrow::Cow;

pub use greybel_symbols::Keyword;

use super::{AnyToken, PendingToken, Token, TokenKind};
use crate::diagnostics::LexerDiagnostic;
use crate::lexer::{CompoundStart, LexResult, Lexer, TokenStream, identifier_end, is_whitespace};
use crate::position::Span;

pub const KEYWORDS: &[Keyword] = &[
    Keyword::If,
    Keyword::In,
    Keyword::Or,
    Keyword::And,
    Keyword::Isa,
    Keyword::For,
    Keyword::Not,
    Keyword::New,
    Keyword::EndIf,
    Keyword::EndFunction,
    Keyword::EndWhile,
    Keyword::EndFor,
    Keyword::Then,
    Keyword::Else,
    Keyword::Break,
    Keyword::While,
    Keyword::ElseIf,
    Keyword::Return,
    Keyword::Function,
    Keyword::Continue,
    Keyword::Repeat,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordToken {
    pub(super) span: Span,
    pub(super) after_space: bool,
    pub(super) keyword: Keyword,
}

impl KeywordToken {
    #[must_use]
    pub fn describe_keyword(keyword: Keyword) -> String {
        format!("Keyword({})", keyword.as_str())
    }
}

impl Token for KeywordToken {
    fn kind(&self) -> TokenKind {
        TokenKind::Keyword
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        self.after_space
    }

    fn value<'s>(&self, _source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed(self.keyword.as_str())
    }

    fn describe(&self, _source: &str) -> String {
        Self::describe_keyword(self.keyword)
    }

    fn keyword(&self) -> Option<Keyword> {
        Some(self.keyword)
    }
}

impl<'src> Lexer<'src> {
    pub(super) fn finish_keyword(
        stream: &TokenStream,
        keyword: Keyword,
        token_start: PendingToken,
    ) -> AnyToken {
        token_start.finish(stream, |span, after_space| KeywordToken {
            span,
            after_space,
            keyword,
        })
    }

    pub(super) fn scan_compound_keyword(
        &self,
        stream: &mut TokenStream,
        start: &CompoundStart,
        token_start: PendingToken,
    ) -> LexResult {
        let bytes = self.source.as_bytes();
        let mut position = stream.cursor.index();

        while bytes.get(position).copied().is_some_and(is_whitespace) {
            position += 1;
        }

        let second_start = position;

        position = identifier_end(bytes, position);

        let second = &self.source[second_start..position];

        if let Some(keyword) = start.keyword_for(second) {
            stream.cursor.set(position);

            return Ok(Self::finish_keyword(stream, keyword, token_start));
        }

        if start.alone {
            return Ok(Self::finish_keyword(stream, start.first, token_start));
        }

        stream.cursor.set(position);

        let followers = start.endings.iter().map(|&(text, _)| text).collect();

        self.raise_at(
            stream,
            token_start,
            LexerDiagnostic::WithoutRelatedKeyword {
                first: start.first.as_str(),
                followers,
            },
        )
    }
}
