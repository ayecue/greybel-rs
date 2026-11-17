use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Api, ApiError};
use crate::store::{WorkspaceRecord, WorkspaceStore};
use crate::workspace::LegacyContent;

/// greybel-editor-be's `/code` API, for greybel-js links and released greybel-vs versions. Projects are stored as
/// workspaces; `content` is `LegacyContent`.
#[derive(Deserialize)]
pub struct CodeRequest {
    content: String,
}

#[derive(Serialize)]
pub struct CodeResponse {
    id: Uuid,
    content: String,
    #[serde(rename = "createdAt")]
    creation_time: DateTime<Utc>,
}

impl From<WorkspaceRecord> for CodeResponse {
    fn from(record: WorkspaceRecord) -> Self {
        Self {
            id: record.id,
            content: LegacyContent::encode(&record.workspace),
            creation_time: record.creation_time,
        }
    }
}

impl<S: WorkspaceStore> Api<S> {
    /// `POST /code {content}`.
    pub(super) async fn save_code(
        State(api): State<Self>,
        body: Result<Json<CodeRequest>, JsonRejection>,
    ) -> Result<Json<CodeResponse>, ApiError> {
        let Json(request) = body?;
        let workspace = LegacyContent::decode(&request.content);

        Ok(Json(api.store_workspace(workspace).await?.into()))
    }

    /// `GET /code/{id}`.
    pub(super) async fn find_code(
        State(api): State<Self>,
        Path(id): Path<String>,
    ) -> Result<Json<CodeResponse>, ApiError> {
        let id = Uuid::parse_str(&id).map_err(|_| ApiError::UnknownId)?;
        let record = api.store.find(id).await?.ok_or(ApiError::UnknownId)?;

        Ok(Json(record.into()))
    }
}
