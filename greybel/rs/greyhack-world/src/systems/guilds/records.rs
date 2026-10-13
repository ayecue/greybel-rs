//! Guilds, company ownership and CrownShield: who owns a company, the guild ranking and the Crown
//! (`Guilds`, `StocksCompany`, `Database.GetGuildRanking`, `NetworkHelperServer.ActivateCrownShieldProtection`).

use crate::{Guild, GuildDraft, GuildFailure, GuildId, GuildsLayout, Result, World};

impl World {
    /// Creates the guild `draft` (guild creation is interface-only in Grey Hack).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn create_guild(
        &mut self,
        draft: &GuildDraft,
    ) -> Result<std::result::Result<GuildId, GuildFailure>> {
        for guild in self.db.guilds().await? {
            if guild.name == draft.name {
                return Ok(Err(GuildFailure::NameInUse(draft.name.clone())));
            }

            if guild.bank_account == draft.bank_account {
                return Ok(Err(GuildFailure::AccountInUse(draft.bank_account.clone())));
            }
        }

        let id = self.db.insert_guild(draft).await?;

        self.crown_ranking_dirty = true;

        Ok(Ok(id))
    }

    /// Copies guilds, company shares and CrownShield networks from Grey Hack into the world, for
    /// mirrored worlds: the guilds exist with the Crown and CrownShield they have, and nothing
    /// restarts. Companies and networks the world lacks are skipped.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn mirror_guilds(&mut self, layout: &GuildsLayout) -> Result<()> {
        for guild in &layout.guilds {
            let id = self.db.insert_guild(&guild.draft).await?;

            self.db.set_guild_crown(id, guild.has_crown).await?;
            self.db
                .set_guild_shield(id, &guild.crown_shield_public_ip)
                .await?;
        }

        for company in &layout.companies {
            self.set_company_seed(&company.public_ip, company.seed)
                .await?;

            for (bank_account, shares) in &company.holders {
                self.set_company_shares(&company.public_ip, bank_account, *shares)
                    .await?;
            }
        }

        for guild in &layout.guilds {
            let Some(row) = self.db.guild_by_name(&guild.draft.name).await? else {
                continue;
            };

            for public_ip in &guild.admin_restore_off {
                if let Some(router) = self.db.router_at(public_ip).await? {
                    self.db
                        .set_guild_admin_restore(row.id, router.network, false)
                        .await?;
                }
            }
        }

        for (public_ip, guild_name) in &layout.shields {
            self.db
                .add_crown_shield_network(public_ip, guild_name)
                .await?;
        }

        self.crown_ranking_dirty = true;

        Ok(())
    }

    /// Returns all guilds in creation order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn guilds(&self) -> Result<Vec<Guild>> {
        self.db.guilds().await
    }
}

#[cfg(test)]
#[path = "tests/records.rs"]
pub(super) mod tests;
