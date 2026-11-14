//! Rental-administrator ownership used by economy pricing (`NetworkLan.superAdminRental`).

use greyhack_world_core::Database;
use greyhack_world_storage::sql::{execute, fetch_value};
use turso::params;

use crate::{NetworkId, PersonId, Result};

/// Queries of the `rental_admins` table owned by economy.
pub(crate) struct RentalRepository {
    db: Database,
}

impl RentalRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn rental_admin(&self, network: NetworkId) -> Result<Option<PersonId>> {
        Ok(fetch_value::<u32>(
            &self.db.connection(),
            "SELECT person_id FROM rental_admins WHERE network_id = ?1",
            params![network.0],
        )
        .await?
        .map(PersonId))
    }

    pub(crate) async fn set_rental_admin(
        &self,
        network: NetworkId,
        admin: Option<PersonId>,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM rental_admins WHERE network_id = ?1",
            params![network.0],
        )
        .await?;
        if let Some(admin) = admin {
            execute(
                &self.db.connection(),
                "INSERT INTO rental_admins (network_id, person_id) VALUES (?1, ?2)",
                params![network.0, admin.0],
            )
            .await?;
        }

        Ok(())
    }
}
