//! The guild ranking and the Crown (`Database.GetGuildRanking`).

use std::collections::HashMap;

use greyhack_behaviour::{
    AdminAbsence, AdminArrival, CROWN_QUALIFICATION_DAYS, DDOS_LEADERBOARD_PENALTY_SECONDS,
    RankedCompany, RankedGuild, SECONDS_PER_DAY, add_days, admin_absent, admin_arrives,
    company_weight, crown_leader, effective_max_invested_companies, guild_points,
    leaderboard_points, max_stocks, next_day, order_companies, sort_leaderboard,
};

use crate::{CrownState, Guild, GuildId, NetworkId, Result, World};

impl World {
    /// Returns the ranking points of `network`'s company (`NetworkLan.GetRankingPoints`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn company_leaderboard_points(&self, network: NetworkId) -> Result<i32> {
        let Some(row) = self.db.network(network).await? else {
            return Ok(0);
        };
        let work = self.company_work_performance(network).await?;
        let price = self.company_share_price(network).await?;

        Ok(leaderboard_points(max_stocks(row.kind), price, work))
    }

    /// Lowers the ranking points of the company at `public_ip` for 24 hours when a guild owns it
    /// (`Database.ApplyDdosRankingPenalty`). Returns whether the penalty applies.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn apply_ddos_leaderboard_penalty(&mut self, public_ip: &str) -> Result<bool> {
        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(false);
        };
        let Some((owner, _)) = self.company_majority_owner(router.network).await? else {
            return Ok(false);
        };

        if self.db.guild_of_account(&owner).await?.is_none() {
            return Ok(false);
        }

        let until = self.time_seconds + DDOS_LEADERBOARD_PENALTY_SECONDS;

        self.db
            .set_company_ddos_until(router.network, until)
            .await?;
        self.crown_ranking_dirty = true;

        Ok(true)
    }

    /// Returns the guild that owns `network`'s company and its absence record for the company's
    /// administrator, or `None` when no guild owns the company.
    async fn owner_guild_absence(
        &self,
        network: NetworkId,
    ) -> Result<Option<(Guild, Option<AdminAbsence>)>> {
        let Some((owner, _)) = self.company_majority_owner(network).await? else {
            return Ok(None);
        };
        let Some(guild) = self.db.guild_of_account(&owner).await? else {
            return Ok(None);
        };
        let absence = self.db.guild_absence(guild.id, network).await?;

        Ok(Some((guild, absence)))
    }

    /// Records that the administrator of `network`'s company is missing for the guild owning it
    /// (`NpcInfo.ApplyAdminAbsencePenalty`).
    pub(crate) async fn apply_admin_absence_penalty(&mut self, network: NetworkId) -> Result<()> {
        let Some((guild, current)) = self.owner_guild_absence(network).await? else {
            return Ok(());
        };
        let today = self.time_seconds / SECONDS_PER_DAY;

        if let Some(record) = admin_absent(current, today) {
            self.db
                .set_guild_absence(guild.id, network, Some(record))
                .await?;
            self.crown_ranking_dirty = true;
        }

        Ok(())
    }

    /// Starts the recovery of the guild's penalty for `network`'s company after an administrator
    /// is hired (`BankHelperServer.HireNpcServerRpc`, `Guild.MarkCompanyAdminHired`).
    pub(crate) async fn mark_company_admin_arrival(&mut self, network: NetworkId) -> Result<()> {
        let Some((guild, current)) = self.owner_guild_absence(network).await? else {
            return Ok(());
        };
        let today = self.time_seconds / SECONDS_PER_DAY;

        match admin_arrives(current, today) {
            AdminArrival::Same => {}
            AdminArrival::Remove => {
                self.db.set_guild_absence(guild.id, network, None).await?;
                self.crown_ranking_dirty = true;
            }
            AdminArrival::Recover(record) => {
                self.db
                    .set_guild_absence(guild.id, network, Some(record))
                    .await?;
                self.crown_ranking_dirty = true;
            }
        }

        Ok(())
    }

    /// Ranks the guilds by the companies they own (`Database.GetGuildRanking`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn guild_leaderboard(&self) -> Result<Vec<RankedGuild>> {
        let today = self.time_seconds / SECONDS_PER_DAY;
        let guilds = self.db.guilds().await?;
        let mut companies: HashMap<GuildId, Vec<(f64, String, RankedCompany)>> = HashMap::new();

        for network in self.db.shared_companies().await? {
            let Some((owner, percent)) = self.company_majority_owner(network).await? else {
                continue;
            };
            let Some(guild) = guilds.iter().find(|guild| guild.bank_account == owner) else {
                continue;
            };
            let points = self.company_leaderboard_points(network).await?;
            let absence = self.db.guild_absence(guild.id, network).await?;
            let ddos = self.db.company_ddos_until(network).await? > self.time_seconds;
            let name = self.company_name(network).await?;

            companies.entry(guild.id).or_default().push((
                company_weight(points, percent),
                name,
                RankedCompany {
                    points,
                    owner_percent: percent,
                    ddos_penalty: ddos,
                    admin_penalty_percent: absence.map_or(0, |record| record.percent_on(today)),
                },
            ));
        }

        let mut leaderboard = Vec::new();

        for guild in &guilds {
            let Some(companies_of_guild) = companies.remove(&guild.id) else {
                continue;
            };

            let companies_of_guild = order_companies(companies_of_guild);
            let max = effective_max_invested_companies(guild.investment_perks, guild.total_exp);

            leaderboard.push(RankedGuild {
                name: guild.name.clone(),
                total_points: guild_points(&companies_of_guild, max),
            });
        }

        sort_leaderboard(&mut leaderboard);

        Ok(leaderboard)
    }

    /// Evaluates the Crown ranking when stocks or guilds changed or its day passed
    /// (`Guilds.UpdateCrownRanking`). Grey Hack runs it every minute.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn update_crown_leaderboard(&mut self) -> Result<()> {
        let now = self.time_seconds;
        let dirty = std::mem::take(&mut self.crown_ranking_dirty);

        if !dirty && now < self.next_crown_evaluation {
            return Ok(());
        }

        self.evaluate_crown_leaderboard(now).await
    }

    /// `Guilds.EvaluateCrownRanking`.
    async fn evaluate_crown_leaderboard(&mut self, now: u64) -> Result<()> {
        let leaderboard = self.guild_leaderboard().await?;
        let leader = crown_leader(&leaderboard).map(|guild| guild.name.clone());
        let guilds = self.db.guilds().await?;
        let mut state = self.db.crown_state().await?;
        let mut changed = false;
        let mut shield_guild = String::new();

        for guild in &guilds {
            if guild.has_crown && leader.as_deref() != Some(guild.name.as_str()) {
                self.db.set_guild_crown(guild.id, false).await?;

                if !guild.crown_shield_public_ip.is_empty() {
                    self.db.set_guild_shield(guild.id, "").await?;
                    shield_guild.clone_from(&guild.name);
                }

                changed = true;
            }
        }

        let mut leader_has_crown = false;

        match leader
            .as_deref()
            .and_then(|name| guilds.iter().find(|guild| guild.name == name))
        {
            None => {
                if !state.candidate.is_empty() || state.since != 0 {
                    state = CrownState::default();
                    changed = true;
                }
            }
            Some(guild) => {
                leader_has_crown = guild.has_crown;

                if !guild.has_crown {
                    if state.candidate != guild.name {
                        state = CrownState {
                            candidate: guild.name.clone(),
                            since: now,
                        };
                        changed = true;
                    } else if now.saturating_sub(state.since)
                        >= CROWN_QUALIFICATION_DAYS * SECONDS_PER_DAY
                    {
                        self.db.set_guild_crown(guild.id, true).await?;
                        leader_has_crown = true;
                        changed = true;
                    }
                }
            }
        }

        if changed {
            if !shield_guild.is_empty() {
                self.db.delete_crown_shield_network(&shield_guild).await?;
            }

            self.db.set_crown_state(&state).await?;
            self.crown_ranking_dirty = true;
        }

        self.next_crown_evaluation = next_day(now);

        if leader.is_some() && !leader_has_crown {
            let crown_day = add_days(state.since, CROWN_QUALIFICATION_DAYS);

            if crown_day < self.next_crown_evaluation {
                self.next_crown_evaluation = crown_day;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/leaderboard.rs"]
mod tests;
