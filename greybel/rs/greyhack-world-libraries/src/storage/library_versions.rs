//! What the world remembers about the versions of its libraries: the saved versions, and the use and patching of the
//! weaknesses of the newest version of each kind (`VersionsControl`).

use greyhack_behaviour::LibraryKind;
use greyhack_world_core::Database;
use greyhack_world_storage::sql::{
    Code, FromRow, RowView, execute, fetch_all, fetch_column, fetch_value,
};
use turso::params;

use crate::{ComputerId, LibraryId, Result};

/// Uses of one weakness of the newest version of a library kind.
pub struct UseCount {
    pub(crate) address: String,
    pub(crate) unsec_value: String,
    pub(crate) uses: u32,
}

impl FromRow for UseCount {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            address: row.get("address")?,
            unsec_value: row.get("unsec_value")?,
            uses: row.get("uses")?,
        })
    }
}

/// Queries of [`Database`] for library versions.
/// Queries of the tables owned by this area.
pub(crate) struct LibraryVersionRepository {
    db: Database,
}

impl LibraryVersionRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn weakness_uses(&self, kind: LibraryKind) -> Result<Vec<UseCount>> {
        Ok(fetch_all(
            &self.db.connection(),
            "SELECT address, unsec_value, uses FROM library_weakness_uses WHERE kind = ?1",
            params![kind.code()],
        )
        .await?)
    }

    pub(crate) async fn weakness_use_count(
        &self,
        kind: LibraryKind,
        address: &str,
        unsec_value: &str,
    ) -> Result<u32> {
        Ok(fetch_value(
            &self.db.connection(),
            "SELECT uses FROM library_weakness_uses WHERE kind = ?1 AND address = ?2 AND unsec_value = ?3",
            params![kind.code(), address, unsec_value],
        )
        .await?
        .unwrap_or(0))
    }

    pub(crate) async fn set_weakness_use_count(
        &self,
        kind: LibraryKind,
        address: &str,
        unsec_value: &str,
        uses: u32,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO library_weakness_uses (kind, address, unsec_value, uses) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (kind, address, unsec_value) DO UPDATE SET uses = excluded.uses",
            params![kind.code(), address, unsec_value, uses],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn replace_weakness_uses(
        &self,
        kind: LibraryKind,
        uses: &[UseCount],
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM library_weakness_uses WHERE kind = ?1",
            params![kind.code()],
        )
        .await?;

        for entry in uses {
            self.set_weakness_use_count(kind, &entry.address, &entry.unsec_value, entry.uses)
                .await?;
        }

        Ok(())
    }

    pub(crate) async fn zone_patches(&self, kind: LibraryKind) -> Result<Vec<(String, u32)>> {
        Ok(fetch_all(
            &self.db.connection(),
            "SELECT address, times_patched FROM library_zone_patches WHERE kind = ?1",
            params![kind.code()],
        )
        .await?)
    }

    pub(crate) async fn replace_zone_patches(
        &self,
        kind: LibraryKind,
        patches: &[(String, u32)],
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM library_zone_patches WHERE kind = ?1",
            params![kind.code()],
        )
        .await?;

        for (address, times) in patches.iter().filter(|(_, times)| *times > 0) {
            execute(
                &self.db.connection(),
                "INSERT INTO library_zone_patches (kind, address, times_patched) VALUES (?1, ?2, ?3)",
                params![kind.code(), address.as_str(), *times],
            )
            .await?;
        }

        Ok(())
    }

    pub(crate) async fn saved_libraries(&self, kind: LibraryKind) -> Result<Vec<LibraryId>> {
        Ok(fetch_column::<u32>(
            &self.db.connection(),
            "SELECT library_id FROM saved_library_versions WHERE kind = ?1 ORDER BY id",
            params![kind.code()],
        )
        .await?
        .into_iter()
        .map(LibraryId)
        .collect())
    }

    pub(crate) async fn saved_versions(&self) -> Result<Vec<(LibraryKind, String)>> {
        Ok(fetch_all::<(Code<LibraryKind>, String)>(
            &self.db.connection(),
            "SELECT saved_library_versions.kind, libraries.version FROM saved_library_versions \
             JOIN libraries ON libraries.id = saved_library_versions.library_id \
             ORDER BY saved_library_versions.id",
            (),
        )
        .await?
        .into_iter()
        .map(|(Code(kind), version)| (kind, version))
        .collect())
    }

    pub(crate) async fn save_library(&self, kind: LibraryKind, library: LibraryId) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO saved_library_versions (kind, library_id) VALUES (?1, ?2)",
            params![kind.code(), library.0],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn prune_saved_libraries(
        &self,
        kind: LibraryKind,
        count: usize,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM saved_library_versions WHERE id IN \
             (SELECT id FROM saved_library_versions WHERE kind = ?1 ORDER BY id LIMIT ?2)",
            params![kind.code(), i64::try_from(count).unwrap_or(i64::MAX)],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn add_library_reporter(&self, player: ComputerId) -> Result<bool> {
        Ok(execute(
            &self.db.connection(),
            "INSERT OR IGNORE INTO library_reporters (player) VALUES (?1)",
            params![player.0],
        )
        .await?
            > 0)
    }

    pub(crate) async fn clear_library_reporters(&self) -> Result<()> {
        execute(&self.db.connection(), "DELETE FROM library_reporters", ()).await?;

        Ok(())
    }

    pub(crate) async fn has_library_check(&self) -> Result<bool> {
        Ok(fetch_value::<i64>(
            &self.db.connection(),
            "SELECT routine_id FROM routine_library_checks LIMIT 1",
            (),
        )
        .await?
        .is_some())
    }
}
