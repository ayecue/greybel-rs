//! Zero-day rumor and library patch-state storage.

use greyhack_behaviour::{LibraryKind, WorldSeed, date_parts};
use greyhack_world_core::{ComputerId, Database, NodeId, Result};
use greyhack_world_storage::sql::{FromRow, RowView, execute, fetch_opt};
use turso::params;

use crate::DebugLibraryState;
use crate::model::zero_day_rumor::ZeroDayRumor;

struct ZeroDayStateRow {
    state: String,
}

impl FromRow for ZeroDayStateRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            state: row.get("state")?,
        })
    }
}

struct DebugLibraryRow {
    computer: ComputerId,
    node: NodeId,
    path: String,
    kind: LibraryKind,
    version: String,
    patch_seed: Option<i64>,
    auto_patch_at: Option<u64>,
    num_patches: u32,
}

impl FromRow for DebugLibraryRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            computer: ComputerId(row.get("computer_id")?),
            node: NodeId(row.get("node_id")?),
            path: row.get("path")?,
            kind: row.get_code("kind")?,
            version: row.get("version")?,
            patch_seed: row.get("patch_seed")?,
            auto_patch_at: row.get("auto_patch_at")?,
            num_patches: row.get("num_patches")?,
        })
    }
}

/// Queries of [`Database`] for debug.
/// Queries of the tables owned by this area.
pub(crate) struct DebugRepository {
    db: Database,
}

impl DebugRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn zero_day_state(&self) -> Result<ZeroDayRumor> {
        let row: Option<ZeroDayStateRow> = fetch_opt(
            &self.db.connection(),
            "SELECT state FROM zero_day_state WHERE id = 1",
            (),
        )
        .await?;

        row.map_or(Ok(ZeroDayRumor::default()), |row| {
            serde_json::from_str(&row.state)
                .map_err(|error| greyhack_world_core::WorldError::InvalidState(error.to_string()))
        })
    }

    pub(crate) async fn save_zero_day_state(&self, state: &ZeroDayRumor) -> Result<()> {
        let state = serde_json::to_string(state)
            .map_err(|error| greyhack_world_core::WorldError::InvalidState(error.to_string()))?;

        execute(
            &self.db.connection(),
            "INSERT INTO zero_day_state (id, state) VALUES (1, ?1) ON CONFLICT (id) DO UPDATE SET state = excluded.state",
            params![state],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn debug_library(
        &self,
        computer: ComputerId,
        node: NodeId,
        world_seed: &WorldSeed,
        now: u64,
    ) -> Result<Option<DebugLibraryState>> {
        let row: Option<DebugLibraryRow> = fetch_opt(
            &self.db.connection(),
            "SELECT nodes.computer_id, nodes.id AS node_id, nodes.path, libraries.kind, libraries.version, \
             debug_libraries.patch_seed, debug_libraries.auto_patch_at, \
             COALESCE(debug_libraries.num_patches, 0) AS num_patches FROM nodes \
             JOIN node_libraries ON node_libraries.node_id = nodes.id \
             JOIN libraries ON libraries.id = node_libraries.library_id \
             LEFT JOIN debug_libraries ON debug_libraries.node_id = nodes.id \
             WHERE nodes.computer_id = ?1 AND nodes.id = ?2",
            params![computer.0, node.0],
        )
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let (year, month, _) = date_parts(now);
        let seed = world_seed.derive_int(
            "zeroday-challenge-v1",
            &[
                &row.kind.code().to_string(),
                &row.version,
                &month.to_string(),
                &year.to_string(),
            ],
        );

        Ok(Some(DebugLibraryState {
            computer: row.computer,
            node: row.node,
            path: row.path,
            seed: i64::from(seed),
            auto_patch_at: row.auto_patch_at,
            num_patches: row.num_patches,
            patch_seed: row.patch_seed,
        }))
    }

    pub(crate) async fn set_debug_library(&self, library: &DebugLibraryState) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO debug_libraries (node_id, patch_seed, auto_patch_at, num_patches) \
             VALUES (?1, ?2, ?3, ?4) ON CONFLICT (node_id) DO UPDATE SET \
             patch_seed = excluded.patch_seed, auto_patch_at = excluded.auto_patch_at, \
             num_patches = excluded.num_patches",
            params![
                library.node.0,
                library.patch_seed,
                library.auto_patch_at.map(u64::cast_signed),
                library.num_patches,
            ],
        )
        .await?;

        Ok(())
    }
}
