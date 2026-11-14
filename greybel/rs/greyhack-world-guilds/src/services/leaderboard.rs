//! The guild ranking and the Crown (`Database.GetGuildRanking`).

use std::collections::HashMap;

use greyhack_behaviour::{
    AdminAbsence, AdminArrival, CROWN_QUALIFICATION_DAYS, DDOS_LEADERBOARD_PENALTY_SECONDS,
    RankedCompany, RankedGuild, SECONDS_PER_DAY, add_days, admin_absent, admin_arrives,
    company_weight, crown_leader, effective_max_invested_companies, guild_points,
    leaderboard_points, max_stocks, next_day, order_companies, sort_leaderboard,
};

use crate::{CrownState, Guild, GuildId, GuildService, NetworkId, Result};

/// Methods of [`World`] for guilds leaderboard.
impl GuildService {
    pub async fn company_leaderboard_points(&self, network: NetworkId) -> Result<i32> {
        let Some(row) = self.networks.network(network).await? else {
            return Ok(0);
        };
        let work = self.company_work_performance(network).await?;
        let price = self.company_share_price(network).await?;

        Ok(leaderboard_points(max_stocks(row.kind), price, work))
    }

    pub async fn apply_ddos_leaderboard_penalty(&self, public_ip: &str) -> Result<bool> {
        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(false);
        };
        let Some((owner, _)) = self.company_majority_owner(router.network).await? else {
            return Ok(false);
        };

        if self.guilds.guild_of_account(&owner).await?.is_none() {
            return Ok(false);
        }

        let until = self.world.time_seconds() + DDOS_LEADERBOARD_PENALTY_SECONDS;

        self.guilds
            .set_company_ddos_until(router.network, until)
            .await?;
        self.world
            .world_mut()
            .state_mut::<crate::CrownRank>()
            .mark_dirty();

        Ok(true)
    }

    pub async fn apply_admin_absence_penalty(&self, network: NetworkId) -> Result<()> {
        let Some((guild, current)) = self.owner_guild_absence(network).await? else {
            return Ok(());
        };
        let today = self.world.time_seconds() / SECONDS_PER_DAY;

        if let Some(record) = admin_absent(current, today) {
            self.guilds
                .set_guild_absence(guild.id, network, Some(record))
                .await?;
            self.world
                .world_mut()
                .state_mut::<crate::CrownRank>()
                .mark_dirty();
        }

        Ok(())
    }

    pub async fn mark_company_admin_arrival(&self, network: NetworkId) -> Result<()> {
        let Some((guild, current)) = self.owner_guild_absence(network).await? else {
            return Ok(());
        };
        let today = self.world.time_seconds() / SECONDS_PER_DAY;

        match admin_arrives(current, today) {
            AdminArrival::Same => {}
            AdminArrival::Remove => {
                self.guilds
                    .set_guild_absence(guild.id, network, None)
                    .await?;
                self.world
                    .world_mut()
                    .state_mut::<crate::CrownRank>()
                    .mark_dirty();
            }
            AdminArrival::Recover(record) => {
                self.guilds
                    .set_guild_absence(guild.id, network, Some(record))
                    .await?;
                self.world
                    .world_mut()
                    .state_mut::<crate::CrownRank>()
                    .mark_dirty();
            }
        }

        Ok(())
    }

    pub async fn guild_leaderboard(&self) -> Result<Vec<RankedGuild>> {
        let today = self.world.time_seconds() / SECONDS_PER_DAY;
        let guilds = self.guilds.guilds().await?;
        let mut companies: HashMap<GuildId, Vec<(f64, String, RankedCompany)>> = HashMap::new();

        for network in self.guilds.shared_companies().await? {
            let Some((owner, percent)) = self.company_majority_owner(network).await? else {
                continue;
            };
            let Some(guild) = guilds.iter().find(|guild| guild.bank_account == owner) else {
                continue;
            };
            let points = self.company_leaderboard_points(network).await?;
            let absence = self.guilds.guild_absence(guild.id, network).await?;
            let ddos = self.guilds.company_ddos_until(network).await? > self.world.time_seconds();
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

    pub async fn update_crown_leaderboard(&self) -> Result<()> {
        let now = self.world.time_seconds();

        if !self
            .world
            .world_mut()
            .state_mut::<crate::CrownRank>()
            .needs_evaluation(now)
        {
            return Ok(());
        }

        self.evaluate_crown_leaderboard(now).await
    }
}

impl GuildService {
    pub async fn owner_guild_absence(
        &self,
        network: NetworkId,
    ) -> Result<Option<(Guild, Option<AdminAbsence>)>> {
        let Some((owner, _)) = self.company_majority_owner(network).await? else {
            return Ok(None);
        };
        let Some(guild) = self.guilds.guild_of_account(&owner).await? else {
            return Ok(None);
        };
        let absence = self.guilds.guild_absence(guild.id, network).await?;

        Ok(Some((guild, absence)))
    }

    pub async fn evaluate_crown_leaderboard(&self, now: u64) -> Result<()> {
        let leaderboard = self.guild_leaderboard().await?;
        let leader = crown_leader(&leaderboard).map(|guild| guild.name.clone());
        let guilds = self.guilds.guilds().await?;
        let mut state = self.guilds.crown_state().await?;
        let mut changed = false;
        let mut shield_guild = String::new();

        for guild in &guilds {
            if guild.has_crown && leader.as_deref() != Some(guild.name.as_str()) {
                self.guilds.set_guild_crown(guild.id, false).await?;

                if !guild.crown_shield_public_ip.is_empty() {
                    self.guilds.set_guild_shield(guild.id, "").await?;
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
                        self.guilds.set_guild_crown(guild.id, true).await?;
                        leader_has_crown = true;
                        changed = true;
                    }
                }
            }
        }

        if changed {
            if !shield_guild.is_empty() {
                self.guilds
                    .delete_crown_shield_network(&shield_guild)
                    .await?;
            }

            self.guilds.set_crown_state(&state).await?;
            self.world
                .world_mut()
                .state_mut::<crate::CrownRank>()
                .mark_dirty();
        }

        let crown_day = (leader.is_some() && !leader_has_crown)
            .then(|| add_days(state.since, CROWN_QUALIFICATION_DAYS));

        self.world
            .world_mut()
            .state_mut::<crate::CrownRank>()
            .schedule(next_day(now), crown_day);

        Ok(())
    }
}
