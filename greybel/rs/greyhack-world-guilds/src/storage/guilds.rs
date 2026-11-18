//! Guild, company share and CrownShield storage.

use greyhack_behaviour::AdminAbsence;
use greyhack_world_core::{Database, NetworkId, Result};
use greyhack_world_storage::sql;
use greyhack_world_storage::sql::{
    FromRow, RowView, execute, fetch_all, fetch_opt, fetch_value, insert,
};
use turso::params;

use crate::{CrownState, Guild, GuildAbsence, GuildDraft, GuildId, Stake};

const GUILD_COLUMNS: &str = "id, name, bank_account, total_exp, investment_perks, admin_restore_perks, \
     has_crown, crown_shield_public_ip, player_member, guild_player_owners.guild_id IS NOT NULL AS player_owner";
const GUILD_FROM: &str =
    "FROM guilds LEFT JOIN guild_player_owners ON guild_player_owners.guild_id = guilds.id";

impl FromRow for Guild {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: GuildId(row.get("id")?),
            name: row.get("name")?,
            bank_account: row.get("bank_account")?,
            total_exp: row.get("total_exp")?,
            investment_perks: row.get("investment_perks")?,
            admin_restore_perks: row.get("admin_restore_perks")?,
            has_crown: row.get("has_crown")?,
            crown_shield_public_ip: row.get("crown_shield_public_ip")?,
            player_member: row.get("player_member")?,
            player_owner: row.get("player_owner")?,
        })
    }
}

impl FromRow for Stake {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            bank_account: row.get("bank_account")?,
            shares: row.get("shares")?,
        })
    }
}

impl FromRow for GuildAbsence {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            network: NetworkId(row.get("network_id")?),
            record: AdminAbsence {
                percent: row.get("percent")?,
                recovery_day: row.get("recovery_day")?,
                recovering: row.get("recovering")?,
            },
        })
    }
}

/// Queries of [`Database`] for guilds.
/// Queries of the tables owned by this area.
pub(crate) struct GuildRepository {
    db: Database,
}

impl GuildRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn insert_guild(&self, draft: &GuildDraft) -> Result<GuildId> {
        let id = insert(
            &self.db.connection(),
            "INSERT INTO guilds (name, bank_account, total_exp, investment_perks, admin_restore_perks, \
             player_member) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                draft.name.as_str(),
                draft.bank_account.as_str(),
                draft.total_exp,
                draft.investment_perks,
                draft.admin_restore_perks,
                draft.player_member
            ],
        )
        .await?;

        let id = GuildId(u32::try_from(id).unwrap_or(u32::MAX));

        if draft.player_owner {
            execute(
                &self.db.connection(),
                "INSERT INTO guild_player_owners (guild_id) VALUES (?1)",
                params![id.0],
            )
            .await?;
        }

        Ok(id)
    }

    pub(crate) async fn guilds(&self) -> Result<Vec<Guild>> {
        Ok(fetch_all(
            &self.db.connection(),
            &format!("SELECT {GUILD_COLUMNS} {GUILD_FROM} ORDER BY id"),
            (),
        )
        .await?)
    }

    pub(crate) async fn guild_by_name(&self, name: &str) -> Result<Option<Guild>> {
        Ok(fetch_opt(
            &self.db.connection(),
            &format!("SELECT {GUILD_COLUMNS} {GUILD_FROM} WHERE name = ?1"),
            params![name],
        )
        .await?)
    }

    pub(crate) async fn guild_of_account(&self, bank_account: &str) -> Result<Option<Guild>> {
        Ok(fetch_opt(
            &self.db.connection(),
            &format!("SELECT {GUILD_COLUMNS} {GUILD_FROM} WHERE bank_account = ?1 ORDER BY id"),
            params![bank_account],
        )
        .await?)
    }

    pub(crate) async fn set_guild_exp_and_member(
        &self,
        guild: GuildId,
        total_exp: u32,
        player_member: bool,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "UPDATE guilds SET total_exp = ?2, player_member = ?3 WHERE id = ?1",
            params![guild.0, total_exp, player_member],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn set_guild_crown(&self, guild: GuildId, has_crown: bool) -> Result<()> {
        execute(
            &self.db.connection(),
            "UPDATE guilds SET has_crown = ?2 WHERE id = ?1",
            params![guild.0, has_crown],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn set_guild_shield(&self, guild: GuildId, public_ip: &str) -> Result<()> {
        execute(
            &self.db.connection(),
            "UPDATE guilds SET crown_shield_public_ip = ?2 WHERE id = ?1",
            params![guild.0, public_ip],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn add_guild_company(
        &self,
        guild: GuildId,
        network: NetworkId,
    ) -> Result<bool> {
        Ok(execute(
            &self.db.connection(),
            "INSERT OR IGNORE INTO guild_companies (guild_id, network_id) VALUES (?1, ?2)",
            params![guild.0, network.0],
        )
        .await?
            > 0)
    }

    pub(crate) async fn remove_guild_company(
        &self,
        guild: GuildId,
        network: NetworkId,
    ) -> Result<bool> {
        Ok(execute(
            &self.db.connection(),
            "DELETE FROM guild_companies WHERE guild_id = ?1 AND network_id = ?2",
            params![guild.0, network.0],
        )
        .await?
            > 0)
    }

    pub(crate) async fn guild_admin_restore_off(&self, guild: GuildId) -> Result<Vec<NetworkId>> {
        let rows = fetch_all::<sql::Scalar<u32>>(
            &self.db.connection(),
            "SELECT network_id FROM guild_admin_restore_disabled WHERE guild_id = ?1 \
             ORDER BY network_id",
            params![guild.0],
        )
        .await?;

        Ok(rows
            .into_iter()
            .map(|sql::Scalar(id)| NetworkId(id))
            .collect())
    }

    pub(crate) async fn set_guild_admin_restore(
        &self,
        guild: GuildId,
        network: NetworkId,
        enabled: bool,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            if enabled {
                "DELETE FROM guild_admin_restore_disabled WHERE guild_id = ?1 AND network_id = ?2"
            } else {
                "INSERT OR IGNORE INTO guild_admin_restore_disabled (guild_id, network_id) \
                 VALUES (?1, ?2)"
            },
            params![guild.0, network.0],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn guild_absence(
        &self,
        guild: GuildId,
        network: NetworkId,
    ) -> Result<Option<AdminAbsence>> {
        let absence = fetch_opt::<GuildAbsence>(
            &self.db.connection(),
            "SELECT network_id, percent, recovery_day, recovering FROM guild_admin_absences \
             WHERE guild_id = ?1 AND network_id = ?2",
            params![guild.0, network.0],
        )
        .await?;

        Ok(absence.map(|absence| absence.record))
    }

    pub(crate) async fn set_guild_absence(
        &self,
        guild: GuildId,
        network: NetworkId,
        record: Option<AdminAbsence>,
    ) -> Result<()> {
        let Some(record) = record else {
            execute(
                &self.db.connection(),
                "DELETE FROM guild_admin_absences WHERE guild_id = ?1 AND network_id = ?2",
                params![guild.0, network.0],
            )
            .await?;

            return Ok(());
        };

        execute(
            &self.db.connection(),
            "INSERT INTO guild_admin_absences (guild_id, network_id, percent, recovery_day, recovering) \
             VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (guild_id, network_id) DO UPDATE SET \
             percent = ?3, recovery_day = ?4, recovering = ?5",
            params![
                guild.0,
                network.0,
                record.percent,
                record.recovery_day.map(u64::cast_signed),
                record.recovering
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn company_shares(&self, network: NetworkId) -> Result<Vec<Stake>> {
        Ok(fetch_all(
            &self.db.connection(),
            "SELECT bank_account, shares FROM company_shares WHERE network_id = ?1 ORDER BY id",
            params![network.0],
        )
        .await?)
    }

    pub(crate) async fn shared_companies(&self) -> Result<Vec<NetworkId>> {
        let rows = fetch_all::<sql::Scalar<u32>>(
            &self.db.connection(),
            "SELECT network_id FROM company_shares GROUP BY network_id ORDER BY MIN(id)",
            (),
        )
        .await?;

        Ok(rows
            .into_iter()
            .map(|sql::Scalar(id)| NetworkId(id))
            .collect())
    }

    pub(crate) async fn set_company_shares(
        &self,
        network: NetworkId,
        bank_account: &str,
        shares: u32,
    ) -> Result<()> {
        if shares == 0 {
            execute(
                &self.db.connection(),
                "DELETE FROM company_shares WHERE network_id = ?1 AND bank_account = ?2",
                params![network.0, bank_account],
            )
            .await?;

            return Ok(());
        }

        execute(
            &self.db.connection(),
            "INSERT INTO company_shares (network_id, bank_account, shares) VALUES (?1, ?2, ?3) \
             ON CONFLICT (network_id, bank_account) DO UPDATE SET shares = ?3",
            params![network.0, bank_account, shares],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn company_seed(&self, network: NetworkId) -> Result<Option<i32>> {
        Ok(fetch_value::<Option<i64>>(
            &self.db.connection(),
            "SELECT seed FROM company_market WHERE network_id = ?1",
            params![network.0],
        )
        .await?
        .flatten()
        .and_then(|seed| i32::try_from(seed).ok()))
    }

    pub(crate) async fn set_company_seed(&self, network: NetworkId, seed: i32) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO company_market (network_id, seed) VALUES (?1, ?2) \
             ON CONFLICT (network_id) DO UPDATE SET seed = ?2",
            params![network.0, i64::from(seed)],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn company_ddos_until(&self, network: NetworkId) -> Result<u64> {
        Ok(fetch_value::<i64>(
            &self.db.connection(),
            "SELECT ddos_until FROM company_market WHERE network_id = ?1",
            params![network.0],
        )
        .await?
        .map_or(0, i64::cast_unsigned))
    }

    pub(crate) async fn set_company_ddos_until(
        &self,
        network: NetworkId,
        until: u64,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO company_market (network_id, ddos_until) VALUES (?1, ?2) \
             ON CONFLICT (network_id) DO UPDATE SET ddos_until = ?2",
            params![network.0, until.cast_signed()],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn is_crown_shield_protected(&self, public_ip: &str) -> Result<bool> {
        if public_ip.is_empty() {
            return Ok(false);
        }

        Ok(fetch_value::<String>(
            &self.db.connection(),
            "SELECT public_ip FROM crown_shield_networks WHERE public_ip = ?1",
            params![public_ip],
        )
        .await?
        .is_some())
    }

    pub(crate) async fn add_crown_shield_network(
        &self,
        public_ip: &str,
        guild_name: &str,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO crown_shield_networks (public_ip, guild_name) VALUES (?1, ?2)",
            params![public_ip, guild_name],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn delete_crown_shield_network(&self, guild_name: &str) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM crown_shield_networks WHERE guild_name = ?1",
            params![guild_name],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn crown_state(&self) -> Result<CrownState> {
        let state = fetch_opt::<CrownRow>(
            &self.db.connection(),
            "SELECT candidate, since FROM crown_state WHERE id = 0",
            (),
        )
        .await?;

        Ok(state.map_or_else(CrownState::default, |row| row.0))
    }

    pub(crate) async fn set_crown_state(&self, state: &CrownState) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT OR REPLACE INTO crown_state (id, candidate, since) VALUES (0, ?1, ?2)",
            params![state.candidate.as_str(), state.since.cast_signed()],
        )
        .await?;

        Ok(())
    }
}

struct CrownRow(CrownState);

impl FromRow for CrownRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self(CrownState {
            candidate: row.get("candidate")?,
            since: row.get("since")?,
        }))
    }
}
