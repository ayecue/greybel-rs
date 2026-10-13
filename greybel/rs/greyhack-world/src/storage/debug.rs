//! Debug account and library patch-state storage.

use greyhack_behaviour::{LibraryKind, WorldSeed, date_parts};
use turso::params;

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_opt};
use crate::{ComputerId, DebugAccount, DebugLibraryState, NodeId, Result};

impl FromRow for DebugAccount {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            user: row.get("user")?,
            password_hash: row.get("password_hash")?,
            failed_logins: row.get("failed_logins")?,
            unit_test_failed: row.get("unit_test_failed")?,
            last_network_ip: row.get("last_network_ip")?,
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
            kind: row.get("kind")?,
            version: row.get("version")?,
            patch_seed: row.get("patch_seed")?,
            auto_patch_at: row.get("auto_patch_at")?,
            num_patches: row.get("num_patches")?,
        })
    }
}

impl Database {
    /// Returns debug account `user`.
    pub(crate) async fn debug_account(&self, user: &str) -> Result<Option<DebugAccount>> {
        Ok(fetch_opt(
            &self.conn,
            "SELECT user, password_hash, failed_logins, unit_test_failed, last_network_ip FROM debug_accounts \
             WHERE user = ?1",
            params![user],
        )
        .await?)
    }

    /// Inserts or replaces `account`.
    pub(crate) async fn set_debug_account(&self, account: &DebugAccount) -> Result<()> {
        execute(
            &self.conn,
            "INSERT OR REPLACE INTO debug_accounts (user, password_hash, failed_logins, unit_test_failed, \
             last_network_ip) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                account.user.as_str(),
                account.password_hash.as_str(),
                account.failed_logins,
                account.unit_test_failed,
                account.last_network_ip.as_deref(),
            ],
        )
        .await?;

        Ok(())
    }

    /// Returns the generated challenge and stored patch state for library `node` on `computer`.
    pub(crate) async fn debug_library(
        &self,
        computer: ComputerId,
        node: NodeId,
        world_seed: &WorldSeed,
        now: u64,
    ) -> Result<Option<DebugLibraryState>> {
        let row: Option<DebugLibraryRow> = fetch_opt(
            &self.conn,
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

    /// Inserts or replaces persistent patch state for `library`.
    pub(crate) async fn set_debug_library(&self, library: &DebugLibraryState) -> Result<()> {
        execute(
            &self.conn,
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
