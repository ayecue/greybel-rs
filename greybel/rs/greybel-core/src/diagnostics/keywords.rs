use crate::error::ErrorCode;
use crate::lexer::Keyword;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum KeywordDiagnostic {
    #[error("unexpected keyword 'null' at start of line")]
    MisplacedNull,
    #[error("unexpected keyword '{}' at start of line", .0.as_str())]
    Misplacement(Keyword),
    #[error("unexpected keyword '{0}' at start of line")]
    Unexpected(String),
    #[error("loop is invalid within single-line 'if'")]
    LoopInSingleLineIf,
    #[error("unmatched block opener")]
    UnmatchedBreak,
    #[error("found = instead of == in if condition")]
    AssignmentInCondition,
    #[error("'{}' without open loop block", .0.as_str())]
    JumpOutsideLoop(Keyword),
}

fn placement_hint(keyword: Keyword) -> Option<&'static str> {
    match keyword {
        Keyword::Function => {
            Some("Use 'function' on the right side of an assignment, e.g. f = function(x)")
        }
        Keyword::New => {
            Some("Use 'new' on the right side of an assignment, e.g. obj = new MyClass")
        }
        Keyword::And => Some("Use 'and' as a binary operator, e.g. if a and b then"),
        Keyword::Or => Some("Use 'or' as a binary operator, e.g. if a or b then"),
        Keyword::Isa => Some("Use 'isa' as a binary operator, e.g. if obj isa MyClass then"),
        Keyword::In => Some("Use 'in' as part of a for loop, e.g. for item in collection"),
        Keyword::Then => Some("Use 'then' after an 'if' condition, e.g. if x > 0 then"),
        Keyword::Repeat => Some("'repeat' is a reserved keyword but has no function"),
        _ => None,
    }
}

impl KeywordDiagnostic {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::JumpOutsideLoop(keyword) if *keyword == Keyword::Break => {
                ErrorCode::BreakOutsideLoop
            }
            Self::UnmatchedBreak => ErrorCode::BreakOutsideLoop,
            Self::JumpOutsideLoop(_) => ErrorCode::ContinueOutsideLoop,
            Self::MisplacedNull
            | Self::Misplacement(_)
            | Self::Unexpected(_)
            | Self::LoopInSingleLineIf
            | Self::AssignmentInCondition => ErrorCode::UnexpectedKeyword,
        }
    }

    #[must_use]
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::MisplacedNull => Some("Use 'null' as a value, e.g. x = null".into()),
            Self::Misplacement(keyword) => placement_hint(*keyword).map(Into::into),
            Self::JumpOutsideLoop(keyword) => Some(format!(
                "'{}' can only be used inside 'while' or 'for' loops",
                keyword.as_str()
            )),
            Self::UnmatchedBreak => {
                Some("'break' can only be used inside 'while' or 'for' loops".to_owned())
            }
            Self::AssignmentInCondition => Some("Compare with '==' instead".to_owned()),
            Self::Unexpected(_) | Self::LoopInSingleLineIf => None,
        }
    }
}
