use crate::type_meta::MAX_TYPE_DEPTH;

#[derive(Debug, thiserror::Error)]
pub enum MetaError {
    #[error("invalid payload: {0}")]
    Payload(#[from] serde_json::Error),
    #[error("no variation group named {group}")]
    UnknownVariationGroup { group: String },
}

pub type Result<T, E = MetaError> = std::result::Result<T, E>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TypeParseError {
    #[error("expected a type name at {position}")]
    ExpectedName { position: usize },
    #[error("expected `{character}` at {position}")]
    MissingChar { character: char, position: usize },
    #[error("unexpected text after the type at {position}")]
    TrailingText { position: usize },
    #[error("types nest deeper than {MAX_TYPE_DEPTH} levels at {position}")]
    DepthLimit { position: usize },
}
