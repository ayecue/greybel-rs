use crate::error::ErrorCode;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LexerDiagnostic {
    #[error("missing closing quote (\")")]
    UnclosedString,
    #[error("Unexpected end of file in multiline comment.")]
    UnclosedBlockComment,
    #[error("invalid numeric literal: {0}")]
    InvalidNumericLiteral(String),
    #[error("'{first}' without related keyword ({}, etc.)", quoted_first_two(.followers))]
    WithoutRelatedKeyword {
        first: &'static str,
        followers: Vec<&'static str>,
    },
}

fn quoted_first_two(words: &[&str]) -> String {
    let quoted_words: Vec<String> = words.iter().take(2).map(|w| format!("'{w}'")).collect();

    quoted_words.join(", ")
}

impl LexerDiagnostic {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::UnclosedString => ErrorCode::UnexpectedStringEof,
            Self::UnclosedBlockComment => ErrorCode::UnclosedBlockComment,
            Self::InvalidNumericLiteral(_) => ErrorCode::InvalidNumericLiteral,
            Self::WithoutRelatedKeyword { .. } => ErrorCode::InvalidEndKeyword,
        }
    }

    #[must_use]
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::UnclosedString => Some("Add a closing quote to terminate the string".into()),
            _ => None,
        }
    }
}
