//! Guilds, company ownership and CrownShield: who owns a company, the guild ranking and the Crown
//! (`Guilds`, `StocksCompany`, `Database.GetGuildRanking`, `NetworkHelperServer.ActivateCrownShieldProtection`).

use crate::{Guild, GuildDraft, GuildFailure, GuildId, GuildService, GuildsLayout, Result};

/// Methods of [`World`] for guilds records.
impl GuildService {
    pub async fn create_guild(
        &self,
        draft: &GuildDraft,
    ) -> Result<std::result::Result<GuildId, GuildFailure>> {
        for guild in self.guilds.guilds().await? {
            if guild.name == draft.name {
                return Ok(Err(GuildFailure::NameInUse(draft.name.clone())));
            }

            if guild.bank_account == draft.bank_account {
                return Ok(Err(GuildFailure::AccountInUse(draft.bank_account.clone())));
            }
        }

        let id = self.guilds.insert_guild(draft).await?;

        self.world
            .world_mut()
            .state_mut::<crate::CrownRank>()
            .mark_dirty();

        Ok(Ok(id))
    }

    pub async fn mirror_guilds(&self, layout: &GuildsLayout) -> Result<()> {
        for guild in &layout.guilds {
            let id = self.guilds.insert_guild(&guild.draft).await?;

            self.guilds.set_guild_crown(id, guild.has_crown).await?;
            self.guilds
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
            let Some(row) = self.guilds.guild_by_name(&guild.draft.name).await? else {
                continue;
            };

            for public_ip in &guild.admin_restore_off {
                if let Some(router) = self.networks.router_at(public_ip).await? {
                    self.guilds
                        .set_guild_admin_restore(row.id, router.network, false)
                        .await?;
                }
            }
        }

        for (public_ip, guild_name) in &layout.shields {
            self.guilds
                .add_crown_shield_network(public_ip, guild_name)
                .await?;
        }

        self.world
            .world_mut()
            .state_mut::<crate::CrownRank>()
            .mark_dirty();

        Ok(())
    }

    pub async fn guilds(&self) -> Result<Vec<Guild>> {
        self.guilds.guilds().await
    }
}
