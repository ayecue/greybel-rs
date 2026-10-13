use gen_lsp_types::{ErrorCodes, LspErrorCodes};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct ResponseError {
    pub code: ErrorCodes,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

pub type Result<T, E = ResponseError> = std::result::Result<T, E>;

impl ResponseError {
    #[must_use]
    pub fn new(code: ErrorCodes, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    #[must_use]
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(ErrorCodes::InvalidParams, message)
    }

    #[must_use]
    pub fn request_failure(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCodes::Custom(LspErrorCodes::RequestFailed.into()),
            message,
        )
    }
}
