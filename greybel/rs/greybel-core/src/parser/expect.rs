use super::Parser;
use crate::diagnostics::ExpectationDiagnostic;
use crate::error::ParseResult;
use crate::lexer::{
    AnyToken, Comment, Eof, Eol, Keyword, KeywordToken, Punct, Punctuator, SliceOperator, TokenKind,
};
use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Expect {
    Punct(Punct),
    Keyword(Keyword),
    Eol,
    Eof,
    SliceOperator,
    Comment,
}

impl Expect {
    #[must_use]
    pub fn matches(self, token: &AnyToken) -> bool {
        match self {
            Expect::Punct(parser) => token.is_punct(parser),
            Expect::Keyword(k) => token.is_keyword(k),
            Expect::Eol => token.is_eol(),
            Expect::Eof => token.is_eof(),
            Expect::SliceOperator => token.kind() == TokenKind::SliceOperator,
            Expect::Comment => token.is_comment(),
        }
    }

    #[must_use]
    pub fn matches_any(group: &[Expect], token: &AnyToken) -> bool {
        group.iter().any(|e| e.matches(token))
    }

    #[must_use]
    pub fn describe(self) -> String {
        match self {
            Expect::Punct(parser) => Punctuator::describe_punct(parser),
            Expect::Keyword(k) => KeywordToken::describe_keyword(k),
            Expect::Eol => Eol::DESCRIPTION.into(),
            Expect::Eof => Eof::DESCRIPTION.into(),
            Expect::SliceOperator => SliceOperator::DESCRIPTION.into(),
            Expect::Comment => Comment::DESCRIPTION.into(),
        }
    }
}

pub mod groups {
    use super::Expect;
    use crate::lexer::Punct;

    pub const BLOCK_END_OF_LINE: &[Expect] = &[Expect::Eol, Expect::Comment];
    pub const STATEMENT_END: &[Expect] = &[Expect::Eol, Expect::Eof, Expect::Comment];
    pub const ASSIGNMENT_END_OF_EXPR: &[Expect] = &[Expect::Eof, Expect::Eol, Expect::Comment];
    pub const COMMAND_ARGUMENTS_END: &[Expect] =
        &[Expect::Punct(Punct::Comma), Expect::Eol, Expect::Eof];
    pub const RETURN_END: &[Expect] = &[Expect::Eol, Expect::Eof, Expect::Comment];
    pub const PARAMETERS_END: &[Expect] = &[Expect::Punct(Punct::RParen), Expect::Eof, Expect::Eol];
    pub const CALL_ARGUMENTS_END: &[Expect] =
        &[Expect::Punct(Punct::Comma), Expect::Punct(Punct::RParen)];
    pub const MAP_SEPARATOR: &[Expect] =
        &[Expect::Punct(Punct::Comma), Expect::Punct(Punct::RBrace)];
    pub const LIST_SEPARATOR: &[Expect] =
        &[Expect::Punct(Punct::Comma), Expect::Punct(Punct::RBracket)];
}

impl Parser<'_> {
    #[must_use]
    pub fn is_at_any(&self, group: &[Expect]) -> bool {
        Expect::matches_any(group, self.token())
    }

    pub fn consume(&mut self, expect: Expect) -> ParseResult<bool> {
        if expect.matches(self.token()) {
            self.advance()?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn require_span(&self, from: Option<u32>) -> Span {
        Span::new(
            from.unwrap_or(self.token().span().start),
            self.token().span().end,
        )
    }

    pub fn require(&mut self, expect: Expect, from: Option<u32>) -> ParseResult<bool> {
        if expect.matches(self.token()) {
            self.advance()?;

            return Ok(true);
        }

        let diagnostic = ExpectationDiagnostic::Token {
            got: self.token().describe(self.source()),
            expectation: expect.describe(),
        };
        let span = self.require_span(from);

        self.raise_at(diagnostic, span)?;

        Ok(false)
    }

    pub fn require_any(&mut self, group: &[Expect], from: Option<u32>) -> ParseResult<bool> {
        if self.is_at_any(group) {
            self.advance()?;

            return Ok(true);
        }

        let diagnostic = ExpectationDiagnostic::OneOf {
            got: self.token().describe(self.source()),
            expectations: group.iter().map(|e| e.describe()).collect(),
        };
        let span = self.require_span(from);

        self.raise_at(diagnostic, span)?;

        Ok(false)
    }
}
