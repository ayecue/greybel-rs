//! Reverse-shell registrations stored by the libraries domain.

use greyhack_world_core::Database;
use greyhack_world_storage::sql::{FromRow, RowView, execute, fetch_all};
use turso::params;

use crate::{ComputerId, Result, ReverseShell, RouterId};

impl FromRow for ReverseShell {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            computer: ComputerId(row.get("computer_id")?),
            port: row.get("port")?,
            user: row.get("user")?,
            pid: row.get("pid")?,
            creation_second: row.get::<i64>("created_at")?.try_into().unwrap_or(0),
        })
    }
}

/// Queries of [`Database`] for reverse-shell registrations.
/// Queries of the tables owned by this area.
pub(crate) struct ReverseShellRepository {
    db: Database,
}

impl ReverseShellRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn reverse_shells(&self, router: RouterId) -> Result<Vec<ReverseShell>> {
        Ok(fetch_all(
            &self.db.connection(),
            "SELECT computer_id, port, user, pid, created_at FROM reverse_shells WHERE router_id = ?1 ORDER BY id",
            params![router.0],
        )
        .await?)
    }

    pub(crate) async fn insert_reverse_shell(
        &self,
        router: RouterId,
        shell: &ReverseShell,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM reverse_shells WHERE router_id = ?1 AND computer_id = ?2 AND pid = ?3",
            params![router.0, shell.computer.0, shell.pid],
        )
        .await?;
        execute(
            &self.db.connection(),
            "INSERT INTO reverse_shells (router_id, computer_id, pid, port, user, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                router.0,
                shell.computer.0,
                shell.pid,
                shell.port,
                shell.user.as_str(),
                greyhack_world_core::storage::seconds(shell.creation_second)
            ],
        )
        .await?;

        Ok(())
    }
}
