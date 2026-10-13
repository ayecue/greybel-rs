use crate::diagnostics::Diagnostic;
use crate::position::{ColumnSettings, LineIndex, Range, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ErrorCode {
    InvalidCharacter = 1001,
    UnexpectedStringEof = 1002,
    InvalidNumericLiteral = 1003,
    InvalidEndKeyword = 1004,
    UnclosedBlockComment = 1005,

    UnexpectedToken = 2001,
    UnexpectedKeyword = 2002,

    ExpectedToken = 3001,
    ExpectedExpression = 3002,
    ExpectedCondition = 3003,

    UnmatchedEnd = 4001,
    UnclosedBlock = 4002,

    BreakOutsideLoop = 5001,
    ContinueOutsideLoop = 5002,
    InvalidDefaultValue = 5003,
    TrailingCommaInParameters = 5004,
    InvalidAssignmentTarget = 5005,
    NodeDepthLimit = 5007,
}

impl ErrorCode {
    #[must_use]
    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    Lexer,
    Parser,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{diagnostic}")]
pub struct ParseError {
    pub kind: ErrorKind,
    pub span: Span,
    pub diagnostic: Diagnostic,
    pub reported_line: Option<u32>,
}

impl ParseError {
    pub fn from_lexer(diagnostic: impl Into<Diagnostic>, span: Span) -> Self {
        Self {
            kind: ErrorKind::Lexer,
            span,
            diagnostic: diagnostic.into(),
            reported_line: None,
        }
    }

    pub fn from_parser(diagnostic: impl Into<Diagnostic>, span: Span) -> Self {
        Self {
            kind: ErrorKind::Parser,
            span,
            diagnostic: diagnostic.into(),
            reported_line: None,
        }
    }

    #[must_use]
    pub fn code(&self) -> ErrorCode {
        self.diagnostic.code()
    }

    #[must_use]
    pub fn message(&self) -> String {
        self.diagnostic.to_string()
    }

    #[must_use]
    pub fn hint(&self) -> Option<String> {
        self.diagnostic.hint()
    }

    #[must_use]
    pub fn range(&self, lines: &LineIndex, columns: ColumnSettings) -> Range {
        lines.range(self.span, columns)
    }
}

pub type ParseResult<T> = Result<T, Box<ParseError>>;
