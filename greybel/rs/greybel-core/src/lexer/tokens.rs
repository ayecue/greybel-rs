use std::iter::FusedIterator;

use super::{LexResult, Lexer, TokenStream};

#[derive(Debug)]
pub struct TokenIter<'l, 'src> {
    lexer: &'l Lexer<'src>,
    stream: TokenStream,
    finished: bool,
}

impl<'l, 'src> TokenIter<'l, 'src> {
    pub(super) fn new(lexer: &'l Lexer<'src>, stream: TokenStream) -> Self {
        Self {
            lexer,
            stream,
            finished: false,
        }
    }

    #[must_use]
    pub fn stream(&self) -> &TokenStream {
        &self.stream
    }
}

impl<'src> Iterator for TokenIter<'_, 'src> {
    type Item = LexResult;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        match self.lexer.next_token(&mut self.stream) {
            Ok(token) if token.is_eof() => {
                self.finished = true;

                None
            }
            Ok(token) => Some(Ok(token)),
            Err(error) => {
                self.finished = true;

                Some(Err(error))
            }
        }
    }
}

impl FusedIterator for TokenIter<'_, '_> {}
