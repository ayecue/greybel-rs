use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Reason {
    #[error("Unterminated [] set.")]
    UnterminatedBracket,
    #[error("Too many )'s.")]
    TooManyParens,
    #[error("Not enough )'s.")]
    NotEnoughParens,
    #[error("Nested quantifier {0}.")]
    NestedQuantify(char),
    #[error("Quantifier {{x,y}} following nothing.")]
    QuantifyAfterNothing,
    #[error("Illegal {{x,y}} with x > y.")]
    IllegalRange,
    #[error("Cannot include class \\{0} in character range.")]
    BadClassInCharRange(char),
    #[error("[x-y] range in reverse order.")]
    ReversedCharRange,
    #[error("(?({0}) ) reference to undefined group.")]
    UndefinedReference(u32),
    #[error("(?({0}) ) malformed.")]
    MalformedReference(u32),
    #[error("Unrecognized grouping construct.")]
    UnrecognizedGroup,
    #[error("Unterminated (?#...) comment.")]
    UnterminatedComment,
    #[error("Illegal \\ at end of pattern.")]
    IllegalEndEscape,
    #[error("Malformed \\k<...> named back reference.")]
    MalformedNameRef,
    #[error("Reference to undefined group number {0}.")]
    UndefinedBackref(u32),
    #[error("Reference to undefined group name {0}.")]
    UndefinedNameRef(String),
    #[error("Insufficient hexadecimal digits.")]
    TooFewHex,
    #[error("Missing control character.")]
    MissingControl,
    #[error("Unrecognized control character.")]
    UnrecognizedControl,
    #[error("Unrecognized escape sequence \\{0}.")]
    UnrecognizedEscape(char),
    #[error("Too many | in (?()|).")]
    TooManyAlternates,
    #[error("Incomplete \\p{{X}} character escape.")]
    IncompleteSlashP,
    #[error("Malformed \\p{{X}} character escape.")]
    MalformedSlashP,
    #[error("Invalid group name: Group names must begin with a word character.")]
    InvalidGroupName,
    #[error("Capture number cannot be zero.")]
    CapnumNotZero,
    #[error("Alternation conditions do not capture and cannot be named.")]
    AlternationCantCapture,
    #[error("Alternation conditions cannot be comments.")]
    AlternationCantHaveComment,
    #[error("Capture group numbers must be less than or equal to Int32.MaxValue.")]
    CaptureGroupOutOfRange,
    #[error("A subtraction must be the last element in a character class.")]
    SubtractionMustBeLast,

    #[error("Balancing groups are not supported.")]
    BalancingGroup,
    #[error("Group {0} is defined more than once, which is not supported.")]
    DuplicateGroup(String),
    #[error("Lone UTF-16 surrogates are not supported.")]
    LoneSurrogate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternError {
    source_text: String,
    pub(super) reason: Reason,
}

impl PatternError {
    pub(super) fn new(source_text: &str, reason: Reason) -> Self {
        Self {
            source_text: source_text.to_owned(),
            reason,
        }
    }
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parsing \"{}\" - {}", self.source_text, self.reason)
    }
}

impl std::error::Error for PatternError {}
