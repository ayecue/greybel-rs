use super::Parser;
use crate::ast::{Comment, NodeId};
use crate::error::ParseResult;
use crate::lexer::{self, Token};
use crate::position::Span;
use crate::syntax::Noop;

impl Comment {
    fn from_token(token: &lexer::Comment, source: &str) -> Self {
        Self {
            span: token.span(),
            text_span: token.text_span(),
            is_multiline: token.text(source).contains('\n'),
        }
    }
}

impl Parser<'_> {
    fn comment(&self) -> Comment {
        let token = self.token().as_comment().expect("the token is a comment");

        Comment::from_token(token, self.source())
    }

    #[must_use]
    pub fn is_on_the_same_line_as_previous(&self) -> bool {
        self.stream.tokens.has_previous()
            && !self
                .lexer
                .has_line_break(self.prev().span().start, self.token().span().start)
    }

    /// Records the comment token in the chunk without attaching it to a statement.
    pub(crate) fn register_comment(&mut self) {
        let comment = self.comment();

        self.stream.chunk_data.current_mut().comments.push(comment);
    }

    fn on_comment(&mut self) {
        let is_statement = !self.is_on_the_same_line_as_previous();
        let comment = self.comment();

        if self.is_preserving() {
            match self.stream.comments.last_statement() {
                Some(last) if !is_statement => {
                    self.stream
                        .nodes
                        .comments_mut(last)
                        .trailing_comments
                        .push(comment.text_span);
                }
                _ => self.stream.comments.push_comment(comment.text_span),
            }
        }

        self.stream.chunk_data.current_mut().comments.push(comment);
    }

    pub fn consume_noop(&mut self) -> ParseResult<()> {
        loop {
            if self.token().is_comment() {
                self.on_comment();
            } else if !self.token().is_eol() {
                return Ok(());
            }

            self.advance()?;
        }
    }

    pub(crate) fn consume_noop_statement(&mut self) -> ParseResult<()> {
        let mut seen_eol = false;

        loop {
            if self.token().is_comment() {
                self.on_comment();
                seen_eol = false;
            } else if self.token().is_eol() {
                if seen_eol && self.is_preserving() {
                    let span = Span::empty(self.token().span().start);
                    let noop = self.alloc(Noop, span);

                    self.stream.blocks.push_statement(noop);
                }

                seen_eol = true;
            } else {
                return Ok(());
            }

            self.advance()?;
        }
    }

    pub fn attach_pending_comments(&mut self, item: NodeId) {
        if !self.is_preserving() {
            return;
        }

        if self.stream.comments.has_lead_comments() {
            let comments = self.stream.comments.take_comments();

            self.stream.nodes.comments_mut(item).lead_comments = comments;
        }

        self.stream.comments.set_last_statement(Some(item));
    }

    pub fn set_last_statement(&mut self, item: NodeId) {
        self.stream.comments.set_last_statement(Some(item));
    }

    pub fn flush_pending_comments(&mut self) {
        if !self.is_preserving() {
            return;
        }

        if self.stream.comments.has_lead_comments() {
            let span = if self.stream.tokens.has_previous() {
                Span::empty(self.prev().span().end)
            } else {
                Span::empty(self.token().span().start)
            };
            let noop = self.alloc(Noop, span);

            self.stream.nodes.comments_mut(noop).lead_comments =
                self.stream.comments.take_comments();
            self.stream.blocks.push_statement(noop);
        }

        self.stream.comments.set_last_statement(None);
    }

    fn has_same_line_comment(&self) -> bool {
        self.is_preserving() && self.token().is_comment() && self.is_on_the_same_line_as_previous()
    }

    pub fn consume_end_trailing_comment(&mut self) -> ParseResult<()> {
        if !self.has_same_line_comment() {
            return Ok(());
        }

        let comment = self.comment();
        let value = comment.text_span;

        self.stream.chunk_data.current_mut().comments.push(comment);
        if let Some(&last) = self.stream.blocks.top_mut().body_mut().last() {
            self.stream
                .nodes
                .comments_mut(last)
                .end_trailing_comments
                .push(value);
        }

        self.advance()
    }

    pub fn consume_inline_comment(&mut self, target: NodeId) -> ParseResult<()> {
        if !self.has_same_line_comment() {
            return Ok(());
        }

        let comment = self.comment();
        let value = comment.text_span;

        self.stream.chunk_data.current_mut().comments.push(comment);
        self.stream
            .nodes
            .comments_mut(target)
            .trailing_comments
            .push(value);

        self.advance()
    }
}
