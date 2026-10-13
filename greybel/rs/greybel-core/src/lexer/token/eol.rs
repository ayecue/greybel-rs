use std::borrow::Cow;

use greybel_symbols::delimiters::STATEMENT_SEPARATOR;

use super::{AnyToken, PendingToken, Token, TokenKind};
use crate::lexer::{Lexer, TokenStream, is_windows_newline};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Eol {
    pub(super) span: Span,
    pub(super) after_space: bool,
}

impl Eol {
    pub const DESCRIPTION: &'static str = "EOL";
}

impl Token for Eol {
    fn kind(&self) -> TokenKind {
        TokenKind::Eol
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        self.after_space
    }

    fn value<'s>(&self, _source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed(STATEMENT_SEPARATOR)
    }

    fn describe(&self, source: &str) -> String {
        let tail = source.get(self.span.start as usize..).unwrap_or_default();
        let text = [STATEMENT_SEPARATOR, "\r\n", "\r", "\n"]
            .into_iter()
            .find(|text| tail.starts_with(text))
            .unwrap_or_default();

        format!("{}({text})", Self::DESCRIPTION)
    }
}

impl<'src> Lexer<'src> {
    pub(crate) fn scan_eol(&self, stream: &mut TokenStream, token_start: PendingToken) -> AnyToken {
        if self.starts_with(stream, STATEMENT_SEPARATOR) {
            stream.cursor.advance(STATEMENT_SEPARATOR.len());

            return token_start.finish(stream, |span, after_space| Eol { span, after_space });
        }

        if is_windows_newline(self.code_at(stream, 0), self.code_at(stream, 1)) {
            stream.cursor.advance(1);
        }

        let token = token_start.finish(stream, |span, after_space| Eol { span, after_space });

        stream.cursor.advance(1);

        token
    }
}
