//! Network administrator rows owned by the NPC domain.

use greyhack_behaviour::{NetworkAdmin, WorkPerformance};
use greyhack_world_core::{Database, NetworkId, PersonId, Result};
use greyhack_world_storage::sql::{FromRow, RowView, execute, fetch_opt, fetch_value};
use turso::params;

struct AdminRow(NetworkAdmin);

impl FromRow for AdminRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        let code: u8 = row.get("work_performance")?;

        Ok(Self(NetworkAdmin {
            intelligence: row.get("intelligence")?,
            pc_experience: row.get("pc_experience")?,
            work_performance: WorkPerformance::from_code(code).ok_or_else(|| {
                turso::Error::ConversionFailure(format!("{code} is no work performance"))
            })?,
            dead_or_fired: row.get("dead_or_fired")?,
        }))
    }
}

/// Queries of [`Database`] for network administrators.
/// Queries of the tables owned by this area.
pub(crate) struct NetworkAdminRepository {
    db: Database,
}

impl NetworkAdminRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn network_admin(&self, network: NetworkId) -> Result<Option<NetworkAdmin>> {
        Ok(fetch_opt::<AdminRow>(
            &self.db.connection(),
            "SELECT intelligence, pc_experience, work_performance, dead_or_fired FROM network_admins \
             WHERE network_id = ?1",
            params![network.0],
        )
        .await?
        .map(|row| row.0))
    }

    pub(crate) async fn network_admin_person(
        &self,
        network: NetworkId,
    ) -> Result<Option<PersonId>> {
        Ok(fetch_value::<Option<u32>>(
            &self.db.connection(),
            "SELECT person_id FROM network_admins WHERE network_id = ?1",
            params![network.0],
        )
        .await?
        .flatten()
        .map(PersonId))
    }

    pub(crate) async fn set_network_admin(
        &self,
        network: NetworkId,
        person: Option<PersonId>,
        admin: &NetworkAdmin,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT OR REPLACE INTO network_admins (network_id, person_id, intelligence, pc_experience, \
             work_performance, dead_or_fired) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                network.0,
                person.map(|person| person.0),
                admin.intelligence,
                admin.pc_experience,
                admin.work_performance as u8,
                admin.dead_or_fired,
            ],
        )
        .await?;

        Ok(())
    }
}
