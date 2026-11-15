//! Library identities keyed by kind and version, and references from files, ports, and router
//! kernels. Exploit content and version-history tables belong to the libraries domain.

use std::cmp::Ordering;

use greyhack_behaviour::{LibraryKind, compare_versions};
use greyhack_world_storage::sql::{Code, execute, fetch_all, fetch_value, insert};
use turso::params;

use super::Database;
use crate::{Library, LibraryId, Result};

/// Queries of the tables owned by this area.
pub(crate) struct LibraryIdentityRepository {
    db: Database,
}

impl LibraryIdentityRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    /// Returns the identity of library `id`.
    pub async fn library_identity(&self, id: LibraryId) -> Result<Option<Library>> {
        let head: Option<(Code<LibraryKind>, String)> = fetch_all(
            &self.db.connection(),
            "SELECT kind, version FROM libraries WHERE id = ?1",
            params![id.0],
        )
        .await?
        .into_iter()
        .next();

        Ok(head.map(|(Code(kind), version)| Library { kind, version }))
    }

    /// Returns library `kind` at `version`.
    pub async fn library_of(&self, kind: LibraryKind, version: &str) -> Result<Option<LibraryId>> {
        Ok(fetch_value::<u32>(
            &self.db.connection(),
            "SELECT id FROM libraries WHERE kind = ?1 AND version = ?2",
            params![kind.code(), version],
        )
        .await?
        .map(LibraryId))
    }

    /// Returns the newest library of `kind`.
    pub async fn latest_library(&self, kind: LibraryKind) -> Result<Option<LibraryId>> {
        Ok(fetch_value::<u32>(
            &self.db.connection(),
            "SELECT library_id FROM library_latest WHERE kind = ?1",
            params![kind.code()],
        )
        .await?
        .map(LibraryId))
    }

    /// Returns the stored identity matching `library`, or inserts its identity if absent.
    pub async fn library_identity_id(&self, library: &Library) -> Result<LibraryId> {
        self.library_row(library.kind, &library.version).await
    }

    /// Returns library `kind` at `version`, inserting it without weaknesses if absent.
    pub async fn library_row(&self, kind: LibraryKind, version: &str) -> Result<LibraryId> {
        match self.library_of(kind, version).await? {
            Some(id) => Ok(id),
            None => self.insert_library_head(kind, version).await,
        }
    }

    /// Inserts library `kind` at `version` and marks it newest unless a newer version exists.
    async fn insert_library_head(&self, kind: LibraryKind, version: &str) -> Result<LibraryId> {
        let id = insert(
            &self.db.connection(),
            "INSERT INTO libraries (kind, version) VALUES (?1, ?2)",
            params![kind.code(), version],
        )
        .await?;
        let id = LibraryId(u32::try_from(id).expect("library ids fit u32"));
        let latest: Option<String> = fetch_value(
            &self.db.connection(),
            "SELECT libraries.version FROM library_latest JOIN libraries ON libraries.id = library_latest.library_id \
             WHERE library_latest.kind = ?1",
            params![kind.code()],
        )
        .await?;

        if latest.is_none_or(|latest| compare_versions(version, &latest) == Ordering::Greater) {
            execute(
                &self.db.connection(),
                "INSERT INTO library_latest (kind, library_id) VALUES (?1, ?2) \
                 ON CONFLICT (kind) DO UPDATE SET library_id = ?2",
                params![kind.code(), id.0],
            )
            .await?;
        }

        Ok(id)
    }
}
