use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use chrono::Utc;
use uuid::Uuid;

use super::{SaveReport, StoreError, WorkspaceRecord, WorkspaceStore};
use crate::workspace::{LegacyContent, Workspace};

/// Workspaces in memory, for running without `DATABASE_URL` and for tests; lost on exit.
#[derive(Clone, Default)]
pub struct MemoryStore {
    table: Arc<Mutex<MemoryTable>>,
}

#[derive(Default)]
struct MemoryTable {
    workspaces: HashMap<Uuid, WorkspaceRecord>,
    ids_by_hash: HashMap<Vec<u8>, Uuid>,
    legacy: HashMap<Uuid, String>,
}

impl MemoryStore {
    /// Adds a project of the legacy `"Code"` table.
    pub fn insert_legacy(&self, id: Uuid, content: &str) {
        self.lock().legacy.insert(id, content.to_owned());
    }

    pub fn workspace_count(&self) -> usize {
        self.lock().workspaces.len()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, MemoryTable> {
        self.table.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl WorkspaceStore for MemoryStore {
    async fn save(&self, workspace: Workspace) -> Result<SaveReport, StoreError> {
        let mut table = self.lock();
        let hash = workspace.content_hash();

        if let Some(record) = table
            .ids_by_hash
            .get(&hash)
            .and_then(|id| table.workspaces.get(id))
        {
            return Ok(SaveReport {
                record: record.clone(),
                is_new: false,
            });
        }

        let record = WorkspaceRecord {
            id: Uuid::new_v4(),
            workspace,
            creation_time: Utc::now(),
        };

        table.ids_by_hash.insert(hash, record.id);
        table.workspaces.insert(record.id, record.clone());

        Ok(SaveReport {
            record,
            is_new: true,
        })
    }

    async fn find(&self, id: Uuid) -> Result<Option<WorkspaceRecord>, StoreError> {
        let table = self.lock();

        if let Some(record) = table.workspaces.get(&id) {
            return Ok(Some(record.clone()));
        }

        Ok(table.legacy.get(&id).map(|content| WorkspaceRecord {
            id,
            workspace: LegacyContent::decode(content),
            creation_time: Utc::now(),
        }))
    }

    async fn check(&self) -> Result<(), StoreError> {
        Ok(())
    }
}
