//! What the third strike does to the player (`Database.GameOver`) and to their session
//! (`PlayerServer.OnDisconnect`, `PlayerHelperServer.RemoveFromWorld`).

use greyhack_behaviour::{SubscriptionKind, exp_after_member_exit};
use greyhack_world_core::{ComputerId, Result};

/// The final strike removed the player from the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameOverEvent;

impl greyhack_world_effects::Event for GameOverEvent {}

/// Methods of [`World`] for player game over.
impl GameOver {
    pub async fn end_game(&self) -> Result<()> {
        let player = self.world.player();
        let account = match self.npcs.person_on(player).await? {
            Some(person) => self.bank.bank_number_of(person).await?,
            None => None,
        };

        self.apply_guild_game_over_penalty().await?;

        if let Some(account) = &account {
            self.restore_stocks_to_company(account).await?;
        }

        // `Database.GameOver`: the computer's rows go, and with them the player's files and their place on
        // the LAN. Its programs end with it.
        self.mine_jobs.cancel_computers(&[player.0]);
        self.routines
            .cancel_computers(&self.world.db().connection(), &[player.0])
            .await?;
        self.world.world_mut().process_ends.insert(player);
        self.computers.remove_computer(player).await?;
        self.world.world_mut().player = ComputerId(0);

        if let Some(account) = &account {
            self.remove_player_payments(account).await?;
        }

        // `PlayerServer.OnDisconnect`
        self.stop_active_traces().await?;

        self.traces.stop_passive_traces().await
    }
}

impl GameOver {
    pub async fn apply_guild_game_over_penalty(&self) -> Result<()> {
        let Some(guild) = self
            .guilds
            .guilds()
            .await?
            .into_iter()
            .find(|guild| guild.player_member)
        else {
            return Ok(());
        };

        self.guilds
            .set_guild_exp_and_member(guild.id, exp_after_member_exit(guild.total_exp), false)
            .await?;
        self.world
            .world_mut()
            .state_mut::<greyhack_world_guilds::CrownRank>()
            .mark_dirty();

        Ok(())
    }

    pub async fn restore_stocks_to_company(&self, account: &str) -> Result<()> {
        for network in self.guilds.shared_companies().await? {
            if !self
                .guilds
                .company_shares(network)
                .await?
                .iter()
                .any(|stake| stake.bank_account == account)
            {
                continue;
            }

            if let Some(router) = self.networks.routers_of(network).await?.first() {
                let public_ip = self
                    .networks
                    .topology
                    .public_ip(router.id.computer())
                    .await?;

                self.guilds
                    .set_company_shares(&public_ip, account, 0)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn remove_player_payments(&self, account: &str) -> Result<()> {
        if let Some(rental) = self
            .bank
            .subscription(account, SubscriptionKind::Rented)
            .await?
        {
            for public_ip in &rental.public_ips {
                self.networks
                    .settings
                    .disable_network(public_ip, true)
                    .await?;
            }
        }

        if let Some(bank) = self
            .bank
            .subscription(account, SubscriptionKind::Bank)
            .await?
        {
            self.bank.store_subscription(&bank, true).await?;
        }

        Ok(())
    }

    pub async fn stop_active_traces(&self) -> Result<()> {
        for trace in self.traces.active_traces().await? {
            self.traces.remove_active_trace(&trace).await?;
        }

        Ok(())
    }
}

/// A world service with only its declared dependencies.
pub struct GameOver {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) npcs: greyhack_world_modules::Dep<greyhack_world_npcs::NpcService>,
    pub(crate) bank: greyhack_world_modules::Dep<greyhack_world_economy::Bank>,
    pub(crate) guilds: greyhack_world_modules::Dep<greyhack_world_guilds::GuildService>,
    pub(crate) traces: greyhack_world_modules::Dep<greyhack_world_traces::TraceService>,
    pub(crate) routines: std::rc::Rc<greyhack_world_effects::Scheduler>,
    pub(crate) mine_jobs: std::rc::Rc<greyhack_world_effects::WorkQueue>,
}

impl GameOver {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        registry: &std::rc::Rc<greyhack_world_modules::Registry>,
    ) -> Self {
        Self {
            world: world.clone(),
            computers: registry.get(),
            networks: registry.get(),
            npcs: registry.get(),
            bank: registry.get(),
            guilds: registry.get(),
            traces: registry.get(),
            routines: registry.scheduler(),
            mine_jobs: registry.works(),
        }
    }
}
