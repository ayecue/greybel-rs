use std::time::Duration;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::store::StoreError;
use crate::workspace::WorkspaceError;

/// An error response: its status and `{"message": ...}`.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    Invalid(#[from] WorkspaceError),
    #[error("{message}")]
    Body { status: StatusCode, message: String },
    #[error("There is no workspace with this id.")]
    UnknownId,
    #[error("Too many saves; try again in {} seconds.", .0.as_secs().max(1))]
    SaveLimit(Duration),
    #[error("The storage failed.")]
    Store(#[from] StoreError),
}

#[derive(Serialize)]
struct ErrorBody {
    message: String,
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        Self::Body {
            status: rejection.status(),
            message: rejection.body_text(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::Invalid(_) => {
                tracing::info!(reason = %self, "workspace rejected");

                StatusCode::BAD_REQUEST
            }
            Self::Body { status, .. } => {
                tracing::info!(reason = %self, "request body rejected");

                *status
            }
            Self::UnknownId => StatusCode::NOT_FOUND,
            Self::SaveLimit(wait) => {
                tracing::warn!(retry_after_s = wait.as_secs().max(1), "save limit reached");

                StatusCode::TOO_MANY_REQUESTS
            }
            Self::Store(error) => {
                tracing::error!(%error, "storage request failed");

                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        let mut response = (
            status,
            Json(ErrorBody {
                message: self.to_string(),
            }),
        )
            .into_response();

        if let Self::SaveLimit(wait) = self
            && let Ok(value) = HeaderValue::from_str(&wait.as_secs().max(1).to_string())
        {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }

        response
    }
}
