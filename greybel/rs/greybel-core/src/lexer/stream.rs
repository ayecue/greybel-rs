use super::AnyToken;
use super::state::{Cursor, LexerErrorSnapshot, LexerErrorStorage, Lookahead, LookaheadSnapshot};
use crate::error::ParseError;

#[derive(Debug, Clone, Default)]
pub struct TokenStream {
    pub cursor: Cursor,
    pub diagnostics: LexerErrorStorage,
    pub lookahead: Lookahead,
}

#[derive(Debug, Clone)]
pub struct TokenSnapshot {
    cursor: Cursor,
    diagnostics: LexerErrorSnapshot,
    lookahead: LookaheadSnapshot,
}

impl TokenStream {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn new_unsafe() -> Self {
        Self {
            diagnostics: LexerErrorStorage::new_unsafe(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn current(&self) -> &AnyToken {
        self.lookahead.current()
    }

    #[must_use]
    pub fn previous(&self) -> &AnyToken {
        self.lookahead.previous()
    }

    #[must_use]
    pub fn has_previous(&self) -> bool {
        self.lookahead.has_previous()
    }

    #[must_use]
    pub fn errors(&self) -> &[ParseError] {
        self.diagnostics.errors()
    }

    pub fn take_errors(&mut self) -> Vec<ParseError> {
        self.diagnostics.take_errors()
    }

    #[must_use]
    pub fn snapshot(&self) -> TokenSnapshot {
        TokenSnapshot {
            cursor: self.cursor,
            diagnostics: self.diagnostics.snapshot(),
            lookahead: self.lookahead.snapshot(),
        }
    }

    pub fn restore(&mut self, snapshot: TokenSnapshot) {
        self.cursor = snapshot.cursor;
        self.diagnostics.restore(snapshot.diagnostics);
        self.lookahead.restore(snapshot.lookahead);
    }
}
