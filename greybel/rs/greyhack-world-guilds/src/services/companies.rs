//! Companies: their shares, owners, seeds, share prices, and admin restore.

use greyhack_behaviour::{
    NO_ADDRESS, average_work_performance, clean_domain_name, company_share_price,
    default_company_seed, effective_max_admin_restore_disabled_companies,
    is_listed_on_stock_market, is_valid_ip, majority_owner, max_stocks,
};
use greyhack_world_core::{NetworkId, Result};

use crate::{AdminRestoreFailure, GuildId, Stake};

/// Methods of [`World`] for guilds companies.
impl GuildService {
    pub async fn set_company_shares(
        &self,
        public_ip: &str,
        bank_account: &str,
        shares: u32,
    ) -> Result<bool> {
        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(false);
        };

        self.guilds
            .set_company_shares(router.network, bank_account, shares)
            .await?;

        if let Some(guild) = self.guilds.guild_of_account(bank_account).await? {
            if shares > 0 {
                self.guilds
                    .add_guild_company(guild.id, router.network)
                    .await?;
            } else if self
                .guilds
                .remove_guild_company(guild.id, router.network)
                .await?
            {
                // `Guild.RemoveInvestedCompany`.
                self.guilds
                    .set_guild_admin_restore(guild.id, router.network, true)
                    .await?;
                self.guilds
                    .set_guild_absence(guild.id, router.network, None)
                    .await?;
            }
        }

        self.world
            .world_mut()
            .state_mut::<crate::CrownRank>()
            .mark_dirty();

        Ok(true)
    }

    pub async fn company_stakes(&self, network: NetworkId) -> Result<Vec<Stake>> {
        self.guilds.company_shares(network).await
    }

    pub async fn company_majority_owner(
        &self,
        network: NetworkId,
    ) -> Result<Option<(String, f32)>> {
        let Some(row) = self.networks.network(network).await? else {
            return Ok(None);
        };
        let holdings = self.guilds.company_shares(network).await?;

        Ok(majority_owner(
            max_stocks(row.kind),
            holdings
                .iter()
                .map(|entry| (entry.bank_account.as_str(), entry.shares)),
        )
        .map(|(owner, percent)| (owner.to_owned(), percent)))
    }

    pub async fn set_company_seed(&self, public_ip: &str, seed: i32) -> Result<bool> {
        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(false);
        };

        self.guilds.set_company_seed(router.network, seed).await?;

        Ok(true)
    }

    pub async fn company_share_price(&self, network: NetworkId) -> Result<i32> {
        let work = self.company_work_performance(network).await?;
        let seed = match self.guilds.company_seed(network).await? {
            Some(seed) => seed,
            None => {
                let public_ip = match self.networks.topology.main_router(network).await? {
                    Some(router) => router.public_ip,
                    None => NO_ADDRESS.to_owned(),
                };

                default_company_seed(&self.world.world_seed(), &public_ip)
            }
        };

        Ok(company_share_price(seed, work))
    }

    pub async fn company_work_performance(&self, network: NetworkId) -> Result<u8> {
        let npcs = self.npcs.npcs_in(network).await?;

        Ok(average_work_performance(
            npcs.len(),
            npcs.iter()
                .filter(|npc| !npc.is_unavailable_for_work())
                .map(|npc| npc.work_performance),
        ))
    }

    pub async fn is_listed_on_stock_market(&self, network: NetworkId) -> Result<bool> {
        Ok(self
            .networks
            .network(network)
            .await?
            .is_some_and(|row| is_listed_on_stock_market(row.kind)))
    }

    pub(crate) async fn sync_company_market(&self, network: NetworkId) -> Result<()> {
        if self.is_listed_on_stock_market(network).await? {
            self.world
                .world_mut()
                .state_mut::<crate::CrownRank>()
                .mark_dirty();
        }

        Ok(())
    }

    pub async fn company_name(&self, network: NetworkId) -> Result<String> {
        let Some(router) = self.networks.topology.main_router(network).await? else {
            return Ok(String::new());
        };

        if router.domain.is_empty() {
            return Ok(router.public_ip);
        }

        Ok(clean_domain_name(&router.domain))
    }

    pub async fn set_admin_restore(
        &self,
        guild_name: &str,
        public_ip: &str,
        enabled: bool,
    ) -> Result<std::result::Result<(), AdminRestoreFailure>> {
        let Some(guild) = self.guilds.guild_by_name(guild_name).await? else {
            return Ok(Err(AdminRestoreFailure::UnknownGuild(
                guild_name.to_owned(),
            )));
        };

        if !is_valid_ip(public_ip) {
            return Ok(Err(AdminRestoreFailure::InvalidCompany));
        }

        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(Err(AdminRestoreFailure::UnknownCompany));
        };
        let owner = self.company_majority_owner(router.network).await?;

        if owner.is_none_or(|(account, _)| account != guild.bank_account) {
            return Ok(Err(AdminRestoreFailure::NotGuildCompany));
        }

        let off_companies = self.guilds.guild_admin_restore_off(guild.id).await?;
        let is_disabled = off_companies.contains(&router.network);

        if enabled {
            self.guilds
                .set_guild_admin_restore(guild.id, router.network, true)
                .await?;
        } else if !is_disabled {
            let max = effective_max_admin_restore_disabled_companies(
                guild.admin_restore_perks,
                guild.total_exp,
            );

            if off_companies.len() >= max as usize {
                return Ok(Err(AdminRestoreFailure::OverLimit));
            }

            self.guilds
                .set_guild_admin_restore(guild.id, router.network, false)
                .await?;
        }

        Ok(Ok(()))
    }

    pub async fn is_admin_restore_disabled(&self, public_ip: &str) -> Result<bool> {
        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(false);
        };
        let Some((account, _)) = self.company_majority_owner(router.network).await? else {
            return Ok(false);
        };
        let Some(guild) = self.guilds.guild_of_account(&account).await? else {
            return Ok(false);
        };

        Ok(self
            .guilds
            .guild_admin_restore_off(guild.id)
            .await?
            .contains(&router.network))
    }
}

/// A world service with only its declared dependencies.
pub struct GuildService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) guilds: std::rc::Rc<crate::storage::GuildRepository>,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) npcs: greyhack_world_modules::Dep<greyhack_world_npcs::NpcService>,
}

impl GuildService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        guilds: std::rc::Rc<crate::storage::GuildRepository>,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
        npcs: greyhack_world_modules::Dep<greyhack_world_npcs::NpcService>,
    ) -> Self {
        Self {
            world: world.clone(),
            guilds,
            computers,
            networks,
            npcs,
        }
    }
}

impl GuildService {
    /// Sets a guild experience total and whether the player is a member.
    pub async fn set_guild_exp_and_member(
        &self,
        guild: GuildId,
        total_exp: u32,
        player_member: bool,
    ) -> Result<()> {
        self.guilds
            .set_guild_exp_and_member(guild, total_exp, player_member)
            .await
    }
}

impl GuildService {
    /// Returns the guild stakes in `network`.
    pub async fn company_shares(&self, network: NetworkId) -> Result<Vec<Stake>> {
        self.guilds.company_shares(network).await
    }
}

impl GuildService {
    /// Returns the networks listed on the stock market.
    pub async fn shared_companies(&self) -> Result<Vec<NetworkId>> {
        self.guilds.shared_companies().await
    }
}
