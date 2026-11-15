use std::borrow::Cow;

use greybel_symbols::delimiters::{BLOCK_COMMENT_END, BLOCK_COMMENT_START, LINE_COMMENT};

use super::{AnyToken, PendingToken, Token, TokenKind};
use crate::diagnostics::LexerDiagnostic;
use crate::lexer::{
    Keyword, LexResult, Lexer, TokenStream, is_end_of_line, is_windows_newline, offset,
};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comment {
    pub(super) span: Span,
    pub(super) after_space: bool,
    pub(super) block: bool,
}

impl Comment {
    pub const DESCRIPTION: &'static str = "Comment";
}

impl Comment {
    #[must_use]
    pub fn text<'s>(&self, source: &'s str) -> &'s str {
        self.text_span().bytes().map_or("", |bytes| &source[bytes])
    }

    #[must_use]
    pub fn text_span(&self) -> Span {
        let (open, close) = if self.block {
            (BLOCK_COMMENT_START.len(), BLOCK_COMMENT_END.len())
        } else {
            (LINE_COMMENT.len(), 0)
        };

        Span::new(
            self.span.start + offset(open),
            self.span.end - offset(close),
        )
    }
}

impl Token for Comment {
    fn kind(&self) -> TokenKind {
        TokenKind::Comment
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        self.after_space
    }

    fn value<'s>(&self, source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed(self.text(source))
    }

    fn describe(&self, _source: &str) -> String {
        Self::DESCRIPTION.into()
    }

    fn as_comment(&self) -> Option<&Comment> {
        Some(self)
    }
}

impl<'src> Lexer<'src> {
    fn finish_comment(stream: &TokenStream, token_start: PendingToken, block: bool) -> AnyToken {
        token_start.finish(stream, |span, after_space| Comment {
            span,
            after_space,
            block,
        })
    }

    #[must_use]
    pub fn is_at_comment(&self, stream: &TokenStream) -> bool {
        self.starts_with(stream, LINE_COMMENT)
            || (self.grammar().lexer.block_comments
                && self.starts_with(stream, BLOCK_COMMENT_START))
    }

    pub fn scan_comment(&self, stream: &mut TokenStream, token_start: PendingToken) -> LexResult {
        let rules = self.grammar().lexer;

        if rules.commented_directives
            && (self.starts_with_word(stream, Keyword::ImportWithComment.as_str())
                || self.starts_with_word(stream, Keyword::IncludeWithComment.as_str()))
        {
            stream.cursor.advance(LINE_COMMENT.len() - 1);

            return self.scan_identifier_or_keyword(stream, token_start);
        }

        if rules.block_comments && self.starts_with(stream, BLOCK_COMMENT_START) {
            return self.scan_block_comment(stream, token_start);
        }

        self.scan_line_comment(stream, token_start)
    }

    pub fn scan_line_comment(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
    ) -> LexResult {
        while !self.is_at_end(stream) && !is_end_of_line(self.code_at(stream, 0)) {
            stream.cursor.advance(1);
        }

        if is_windows_newline(self.code_at(stream, 0), self.code_at(stream, 1)) {
            stream.cursor.advance(1);
        }

        Ok(Self::finish_comment(stream, token_start, false))
    }

    pub fn scan_block_comment(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
    ) -> LexResult {
        stream.cursor.advance(BLOCK_COMMENT_START.len());

        let mut closed = false;

        while !self.is_at_end(stream) {
            if self.starts_with(stream, BLOCK_COMMENT_END) {
                closed = true;
                break;
            }

            stream.cursor.advance(1);
        }

        if !closed {
            return self.raise_at(stream, token_start, LexerDiagnostic::UnclosedBlockComment);
        }

        stream.cursor.advance(BLOCK_COMMENT_END.len());

        Ok(Self::finish_comment(stream, token_start, true))
    }
}

#[cfg(test)]
#[path = "tests/comment.rs"]
mod tests;
