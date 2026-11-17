use std::collections::BTreeMap;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Api, ApiError};
use crate::store::{WorkspaceRecord, WorkspaceStore};
use crate::workspace::Workspace;

#[derive(Deserialize)]
pub struct WorkspaceRequest {
    entry: String,
    files: BTreeMap<String, String>,
}

#[derive(Serialize)]
pub struct WorkspaceResponse {
    id: Uuid,
    entry: String,
    files: BTreeMap<String, String>,
    #[serde(rename = "createdAt")]
    creation_time: DateTime<Utc>,
}

impl From<WorkspaceRecord> for WorkspaceResponse {
    fn from(record: WorkspaceRecord) -> Self {
        Self {
            id: record.id,
            entry: record.workspace.entry,
            files: record.workspace.files,
            creation_time: record.creation_time,
        }
    }
}

impl<S: WorkspaceStore> Api<S> {
    /// `POST /workspaces {entry, files}`: stores the workspace, or returns the stored one with the same files.
    pub(super) async fn save_workspace(
        State(api): State<Self>,
        body: Result<Json<WorkspaceRequest>, JsonRejection>,
    ) -> Result<Json<WorkspaceResponse>, ApiError> {
        let Json(request) = body?;
        let workspace = Workspace {
            entry: request.entry,
            files: request.files,
        };

        Ok(Json(api.store_workspace(workspace).await?.into()))
    }

    /// Validates and stores `workspace`, and logs the save without file contents.
    pub(super) async fn store_workspace(
        &self,
        workspace: Workspace,
    ) -> Result<WorkspaceRecord, ApiError> {
        workspace.validate()?;

        let characters = workspace.count_characters();
        let report = self.store.save(workspace).await?;

        tracing::info!(
            id = %report.record.id,
            files = report.record.workspace.files.len(),
            characters,
            is_new = report.is_new,
            "workspace saved"
        );

        Ok(report.record)
    }

    /// `GET /workspaces/{id}`: a workspace, or a project saved through the legacy `/code` API.
    pub(super) async fn find_workspace(
        State(api): State<Self>,
        Path(id): Path<String>,
    ) -> Result<Json<WorkspaceResponse>, ApiError> {
        let id = Uuid::parse_str(&id).map_err(|_| ApiError::UnknownId)?;
        let record = api.store.find(id).await?.ok_or(ApiError::UnknownId)?;

        Ok(Json(record.into()))
    }
}
