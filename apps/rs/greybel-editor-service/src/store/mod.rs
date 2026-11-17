mod memory;
mod postgres;

use std::future::Future;

use chrono::{DateTime, Utc};
pub use memory::MemoryStore;
pub use postgres::PostgresStore;
use uuid::Uuid;

use crate::workspace::Workspace;

/// A stored workspace, with its id and when it was first saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRecord {
    pub id: Uuid,
    pub workspace: Workspace,
    pub creation_time: DateTime<Utc>,
}

/// The workspace `save` returns, and whether it is new or one stored before with the same entry and files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveReport {
    pub record: WorkspaceRecord,
    pub is_new: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

/// Storage for shared workspaces, including those of the legacy `"Code"` table.
pub trait WorkspaceStore: Clone + Send + Sync + 'static {
    /// Stores `workspace`, or returns the stored workspace with the same entry and files.
    fn save(
        &self,
        workspace: Workspace,
    ) -> impl Future<Output = Result<SaveReport, StoreError>> + Send;

    /// The workspace `id`, or the legacy project `id` read as a workspace.
    fn find(
        &self,
        id: Uuid,
    ) -> impl Future<Output = Result<Option<WorkspaceRecord>, StoreError>> + Send;

    /// Succeeds when the storage answers.
    fn check(&self) -> impl Future<Output = Result<(), StoreError>> + Send;
}
