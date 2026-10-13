use greybel_interpreter::NativeError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IntrinsicError {
    #[error("Index Error: index ({index}) out of range ({min} to {max})")]
    IndexOutOfRange { index: i64, min: i64, max: i64 },
    #[error(
        "Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')"
    )]
    NegativeSortKey,
    #[error("char: invalid char code.")]
    InvalidCharCode,
    #[error("code: empty string")]
    EmptyCode,
    #[error("insert: index argument required")]
    InsertIndexMissing,
    #[error("insert: number required for index argument")]
    InsertIndexNotANumber,
    #[error("insert called on invalid type")]
    InsertType,
    #[error("argument to 'remove' must not be null")]
    RemoveNull,
    #[error("Type Error: 'remove' requires map, list, or string")]
    RemoveType,
    #[error("argument to 'replace' must not be null")]
    ReplaceNull,
    #[error("replace: oldval argument is empty")]
    ReplaceEmpty,
    #[error("Type Error: 'replace' requires map, list, or string")]
    ReplaceType,
    #[error("range() error (step==0)")]
    RangeStepZero,
    #[error("range() error")]
    Range,
    #[error("list too large")]
    ListTooLarge,
    #[error("Function does not accept floating point Not-a-Number values.")]
    NotANumber,
}

impl NativeError for IntrinsicError {}
