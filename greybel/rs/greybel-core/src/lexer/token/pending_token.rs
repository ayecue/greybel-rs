use super::AnyToken;
use crate::lexer::{Lexer, TokenStream, offset};
use crate::position::Span;

#[derive(Debug)]
pub struct PendingToken {
    pub(super) start: usize,
    pub(super) after_space: bool,
}

impl PendingToken {
    pub(crate) fn new(start: usize, after_space: bool) -> Self {
        Self { start, after_space }
    }

    #[must_use]
    pub fn start(&self) -> usize {
        self.start
    }

    #[must_use]
    pub fn after_space(&self) -> bool {
        self.after_space
    }

    #[must_use]
    pub fn span(&self, stream: &TokenStream) -> Span {
        Span::new(offset(self.start), offset(stream.cursor.index()))
    }

    #[must_use]
    pub fn text<'src>(&self, lexer: &Lexer<'src>, stream: &TokenStream) -> &'src str {
        &lexer.source()[self.start..stream.cursor.index()]
    }

    pub fn finish<T>(self, stream: &TokenStream, build: impl FnOnce(Span, bool) -> T) -> AnyToken
    where
        T: Into<AnyToken>,
    {
        build(self.span(stream), self.after_space).into()
    }
}
