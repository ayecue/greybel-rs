//! Smart-appliance, traffic-network, and traffic-cooldown persistence.

use greyhack_behaviour::SmartApplianceSettings;
use greyhack_world_core::{ComputerId, Database, NetworkId, PersonId, Result};
use greyhack_world_storage::sql::{FromRow, RowView, execute, fetch_opt, fetch_value};
use turso::params;

use crate::TrafficNet;

struct SmartRow(SmartApplianceSettings);

impl FromRow for SmartRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self(SmartApplianceSettings {
            model: row.get("model")?,
            required_power: row.get::<i64>("required_power")? as i32,
            required_temperature: row.get::<i64>("required_temperature")? as i32,
            power: row.get::<i64>("power")? as i32,
            temperature: row.get::<i64>("temperature")? as i32,
            alarm: row.get("alarm")?,
            override_enabled: row.get("override_enabled")?,
            recovery_at: row.get::<i64>("broken_until")?.try_into().unwrap_or(0),
        }))
    }
}

impl FromRow for TrafficNet {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            credentials: row.get::<Option<u32>>("credentials_id")?.map(PersonId),
            override_enabled: row.get("override_enabled")?,
            override_used: row.get("override_used")?,
        })
    }
}

/// Queries of the smart-device tables.
pub(crate) struct SmartDeviceRepository {
    db: Database,
}

impl SmartDeviceRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn smart(
        &self,
        computer: ComputerId,
    ) -> Result<Option<SmartApplianceSettings>> {
        Ok(fetch_opt::<SmartRow>(
            &self.db.connection(),
            "SELECT model, required_power, required_temperature, power, temperature, alarm, override_enabled, broken_until \
             FROM smart_appliances WHERE computer_id = ?1",
            params![computer.0],
        )
        .await?
        .map(|row| row.0))
    }

    pub(crate) async fn set_smart(
        &self,
        computer: ComputerId,
        smart: Option<&SmartApplianceSettings>,
    ) -> Result<()> {
        let Some(smart) = smart else {
            execute(
                &self.db.connection(),
                "DELETE FROM smart_appliances WHERE computer_id = ?1",
                params![computer.0],
            )
            .await?;

            return Ok(());
        };

        execute(
            &self.db.connection(),
            "INSERT OR REPLACE INTO smart_appliances (computer_id, model, required_power, required_temperature, power, \
             temperature, alarm, override_enabled, broken_until) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                computer.0,
                smart.model.as_str(),
                smart.required_power,
                smart.required_temperature,
                smart.power,
                smart.temperature,
                smart.alarm,
                smart.override_enabled,
                greyhack_world_core::storage::seconds(smart.recovery_at),
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn traffic(&self, network: NetworkId) -> Result<TrafficNet> {
        Ok(fetch_opt(
            &self.db.connection(),
            "SELECT credentials_id, override_enabled, override_used FROM traffic_networks WHERE network_id = ?1",
            params![network.0],
        )
        .await?
        .unwrap_or_default())
    }

    pub(crate) async fn set_traffic(&self, network: NetworkId, traffic: &TrafficNet) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO traffic_networks (network_id, credentials_id, override_enabled, override_used) \
             VALUES (?1, ?2, ?3, ?4) ON CONFLICT (network_id) DO UPDATE SET credentials_id = ?2, \
             override_enabled = ?3, override_used = ?4",
            params![
                network.0,
                traffic.credentials.map(|person| person.0),
                traffic.override_enabled,
                traffic.override_used,
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn traffic_cooldown(&self) -> Result<Option<u64>> {
        Ok(fetch_value::<Option<i64>>(
            &self.db.connection(),
            "SELECT cooldown_until FROM traffic_state WHERE id = 0",
            (),
        )
        .await?
        .flatten()
        .map(|seconds| seconds.try_into().unwrap_or(0)))
    }

    pub(crate) async fn set_traffic_cooldown(&self, cooldown: Option<u64>) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO traffic_state (id, cooldown_until) VALUES (0, ?1) \
             ON CONFLICT (id) DO UPDATE SET cooldown_until = ?1",
            params![cooldown.map(|seconds| i64::try_from(seconds).unwrap_or(i64::MAX))],
        )
        .await?;

        Ok(())
    }
}
