mod blocks;
mod depth;
mod expectation;
mod functions;
mod greybel;
mod greyscript;
mod keywords;
mod lexer;
mod statements;

pub use blocks::BlockDiagnostic;
pub use depth::DepthDiagnostic;
pub use expectation::ExpectationDiagnostic;
pub use functions::FunctionDiagnostic;
pub use greybel::GreybelDiagnostic;
pub use greyscript::GreyScriptDiagnostic;
pub use keywords::KeywordDiagnostic;
pub use lexer::LexerDiagnostic;
pub use statements::StatementDiagnostic;

use crate::error::{ErrorCode, ErrorKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportedLine {
    Statement,
    Token,
    End,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Diagnostic {
    #[error(transparent)]
    Lexer(#[from] LexerDiagnostic),
    #[error(transparent)]
    Expectation(#[from] ExpectationDiagnostic),
    #[error(transparent)]
    Block(#[from] BlockDiagnostic),
    #[error(transparent)]
    Keyword(#[from] KeywordDiagnostic),
    #[error(transparent)]
    Function(#[from] FunctionDiagnostic),
    #[error(transparent)]
    Statement(#[from] StatementDiagnostic),
    #[error(transparent)]
    Depth(#[from] DepthDiagnostic),
    #[error(transparent)]
    Greybel(#[from] GreybelDiagnostic),
    #[error(transparent)]
    GreyScript(#[from] GreyScriptDiagnostic),
}

impl Diagnostic {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Lexer(d) => d.code(),
            Self::Expectation(d) => d.code(),
            Self::Block(d) => d.code(),
            Self::Keyword(d) => d.code(),
            Self::Function(d) => d.code(),
            Self::Statement(d) => d.code(),
            Self::Depth(d) => d.code(),
            Self::Greybel(d) => d.code(),
            Self::GreyScript(d) => d.code(),
        }
    }

    #[must_use]
    pub fn reported_kind(&self) -> ErrorKind {
        match self {
            Self::Lexer(
                LexerDiagnostic::UnclosedString | LexerDiagnostic::WithoutRelatedKeyword { .. },
            ) => ErrorKind::Lexer,
            _ => ErrorKind::Parser,
        }
    }

    #[must_use]
    pub fn reported_line(&self) -> ReportedLine {
        match self {
            Self::Expectation(
                ExpectationDiagnostic::Token { .. }
                | ExpectationDiagnostic::OneOf { .. }
                | ExpectationDiagnostic::Identifier { .. }
                | ExpectationDiagnostic::Parameter { .. },
            )
            | Self::Keyword(
                KeywordDiagnostic::Unexpected(_) | KeywordDiagnostic::JumpOutsideLoop(_),
            )
            | Self::Statement(StatementDiagnostic::ForWithoutSequence) => ReportedLine::Token,
            Self::Block(BlockDiagnostic::MissingEnd { .. })
            | Self::Keyword(KeywordDiagnostic::UnmatchedBreak) => ReportedLine::End,
            _ => ReportedLine::Statement,
        }
    }

    #[must_use]
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::Lexer(d) => d.hint(),
            Self::Expectation(d) => d.hint(),
            Self::Block(d) => d.hint(),
            Self::Keyword(d) => d.hint(),
            Self::Function(d) => d.hint(),
            Self::Statement(d) => d.hint(),
            Self::Depth(d) => d.hint(),
            Self::Greybel(_) | Self::GreyScript(_) => None,
        }
    }
}
