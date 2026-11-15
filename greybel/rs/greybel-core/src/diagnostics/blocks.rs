use std::fmt::Write as _;

use crate::error::ErrorCode;
use crate::lexer::Keyword;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BlockDiagnostic {
    #[error("'{end}' without matching '{block}'")]
    UnmatchedEnd { end: String, block: &'static str },
    #[error("'{0}' without matching block starter")]
    WithoutStarter(&'static str),
    #[error("{}", unclosed_message(.block, *.awaits_end))]
    MissingEnd {
        block: &'static str,
        awaits_end: bool,
        inside: Option<(&'static str, u32)>,
    },
}

fn unclosed_message(block: &str, awaits_end: bool) -> String {
    if awaits_end {
        format!(
            "'{block}' without matching '{} {block}'",
            Keyword::End.as_str()
        )
    } else {
        "unmatched block opener".to_owned()
    }
}

impl BlockDiagnostic {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::UnmatchedEnd { .. } | Self::WithoutStarter(_) => ErrorCode::UnmatchedEnd,
            Self::MissingEnd { .. } => ErrorCode::UnclosedBlock,
        }
    }

    #[must_use]
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::UnmatchedEnd { .. } | Self::WithoutStarter(_) => None,
            Self::MissingEnd { block, inside, .. } => {
                let mut hint = format!("Add 'end {block}' to close this block");

                if let Some((parent, line)) = inside {
                    let _ = write!(hint, " (inside '{parent}' at line {line})");
                }

                Some(hint)
            }
        }
    }
}
