//! Companies: their shares, owners, seeds, share prices, and admin restore.

use greyhack_behaviour::{
    NO_ADDRESS, average_work_performance, clean_domain_name, company_share_price,
    default_company_seed, effective_max_admin_restore_disabled_companies,
    is_listed_on_stock_market, is_valid_ip, majority_owner, max_stocks,
};

use crate::{AdminRestoreFailure, NetworkId, Result, World};

impl World {
    /// Returns the shares `bank_account` has of the company at `public_ip`, set as a purchase
    /// would (`StocksCompany.BuyStocksCompany`, `BankHelperServer.PurchaseStockServerRpc`): a
    /// guild's first shares register the company as one it invested in, and the last shares sold
    /// remove it (`StocksCompany.RemoveStocks`). Payment and share prices are not applied. Returns
    /// false without a network at `public_ip`.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_company_shares(
        &mut self,
        public_ip: &str,
        bank_account: &str,
        shares: u32,
    ) -> Result<bool> {
        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(false);
        };

        self.db
            .set_company_shares(router.network, bank_account, shares)
            .await?;

        if let Some(guild) = self.db.guild_of_account(bank_account).await? {
            if shares > 0 {
                self.db.add_guild_company(guild.id, router.network).await?;
            } else if self
                .db
                .remove_guild_company(guild.id, router.network)
                .await?
            {
                // `Guild.RemoveInvestedCompany`.
                self.db
                    .set_guild_admin_restore(guild.id, router.network, true)
                    .await?;
                self.db
                    .set_guild_absence(guild.id, router.network, None)
                    .await?;
            }
        }

        self.crown_ranking_dirty = true;

        Ok(true)
    }

    /// Returns the bank account owning more than half of `network`'s company and the percent it
    /// owns (`Database.GetCompanyMajorityOwnerBankAccount`, `StocksCompany.TryGetMajorityOwner`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn company_majority_owner(
        &self,
        network: NetworkId,
    ) -> Result<Option<(String, f32)>> {
        let Some(row) = self.db.network(network).await? else {
            return Ok(None);
        };
        let holdings = self.db.company_shares(network).await?;

        Ok(majority_owner(
            max_stocks(row.kind),
            holdings
                .iter()
                .map(|entry| (entry.bank_account.as_str(), entry.shares)),
        )
        .map(|(owner, percent)| (owner.to_owned(), percent)))
    }

    /// Sets the seed the share price of the network at `public_ip` is drawn from (`NetworkLan.seed`),
    /// as copied from Grey Hack for mirrored worlds. Returns false without a network.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_company_seed(&mut self, public_ip: &str, seed: i32) -> Result<bool> {
        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(false);
        };

        self.db.set_company_seed(router.network, seed).await?;

        Ok(true)
    }

    /// Returns the current share price of `network`'s company (`NetworkLan.GetPriceRecords`): drawn
    /// from the network's seed and its work performance. Grey Hack stores a history it refreshes
    /// when someone looks, which the world does not.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn company_share_price(&self, network: NetworkId) -> Result<i32> {
        let work = self.company_work_performance(network).await?;
        let seed = match self.db.company_seed(network).await? {
            Some(seed) => seed,
            None => {
                let public_ip = match self.main_router(network).await? {
                    Some(router) => router.public_ip,
                    None => NO_ADDRESS.to_owned(),
                };

                default_company_seed(&self.world_seed, &public_ip)
            }
        };

        Ok(company_share_price(seed, work))
    }

    /// Returns the average work performance code of `network`'s employees who are not gone
    /// (`NetworkLan.GetWorkPerformance`).
    pub(super) async fn company_work_performance(&self, network: NetworkId) -> Result<u8> {
        let npcs = self.db.npcs_in(network).await?;

        Ok(average_work_performance(
            npcs.len(),
            npcs.iter()
                .filter(|npc| !npc.is_unavailable_for_work())
                .map(|npc| npc.work_performance),
        ))
    }

    /// Whether companies of `network` are listed on the stock market
    /// (`StocksCompany.IsListedOnStockMarket`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_listed_on_stock_market(&self, network: NetworkId) -> Result<bool> {
        Ok(self
            .db
            .network(network)
            .await?
            .is_some_and(|row| is_listed_on_stock_market(row.kind)))
    }

    /// Returns the company name the ranking sorts by (`StocksCompany.GetCompanyName`).
    pub(super) async fn company_name(&self, network: NetworkId) -> Result<String> {
        let Some(router) = self.main_router(network).await? else {
            return Ok(String::new());
        };

        if router.domain.is_empty() {
            return Ok(router.public_ip);
        }

        Ok(clean_domain_name(&router.domain))
    }

    /// Turns Admin Restore off or on for the company at `public_ip` as the guild named `guild_name`
    /// (`NetworkHelperServer.AdminCompanySetAdminRestoreServerRpc`, `Guild.SetAdminRestoreEnabled`). The
    /// window and the permission of the player's guild role are not modelled: the guild acts.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_admin_restore(
        &mut self,
        guild_name: &str,
        public_ip: &str,
        enabled: bool,
    ) -> Result<std::result::Result<(), AdminRestoreFailure>> {
        let Some(guild) = self.db.guild_by_name(guild_name).await? else {
            return Ok(Err(AdminRestoreFailure::UnknownGuild(
                guild_name.to_owned(),
            )));
        };

        if !is_valid_ip(public_ip) {
            return Ok(Err(AdminRestoreFailure::InvalidCompany));
        }

        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(Err(AdminRestoreFailure::UnknownCompany));
        };
        let owner = self.company_majority_owner(router.network).await?;

        if owner.is_none_or(|(account, _)| account != guild.bank_account) {
            return Ok(Err(AdminRestoreFailure::NotGuildCompany));
        }

        let off_companies = self.db.guild_admin_restore_off(guild.id).await?;
        let is_disabled = off_companies.contains(&router.network);

        if enabled {
            self.db
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

            self.db
                .set_guild_admin_restore(guild.id, router.network, false)
                .await?;
        }

        Ok(Ok(()))
    }

    /// Returns whether the guild that owns the company at `public_ip` turned Admin Restore off for it
    /// (`Guilds.IsAdminRestoreOffForCompany`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_admin_restore_disabled(&self, public_ip: &str) -> Result<bool> {
        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(false);
        };
        let Some((account, _)) = self.company_majority_owner(router.network).await? else {
            return Ok(false);
        };
        let Some(guild) = self.db.guild_of_account(&account).await? else {
            return Ok(false);
        };

        Ok(self
            .db
            .guild_admin_restore_off(guild.id)
            .await?
            .contains(&router.network))
    }
}

#[cfg(test)]
#[path = "tests/companies.rs"]
mod tests;
