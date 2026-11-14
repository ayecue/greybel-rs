//! Active traces: administrators tracing the player while they act on a company's computers, and
//! the strikes that follow (`ActiveTraceSystem`, `PlayerHelperServer.ActiveGameOver`).
//!
//! Grey Hack runs traces in real time; the world runs them in world time, one real second being
//! [`greyhack_behaviour::TIME_SCALE`] world seconds.

use greyhack_behaviour::{
    LogAction, NetworkKind, active_trace_end, add_strike, should_start_tracking, token_trace,
};
use greyhack_world_npcs::Npc;

use crate::model::ActiveTrace;
use crate::{ComputerId, Result, StrikeEvent, TraceKind};

/// Methods of [`World`] for guilds active traces.
impl Traces {
    pub async fn player_token_trace(&self) -> Result<String> {
        Ok(token_trace(
            self.world.seed(),
            self.traces.token_generation().await?,
        ))
    }

    pub async fn active_traces(&self) -> Result<Vec<ActiveTrace>> {
        self.traces.active_traces().await
    }

    pub async fn passive_traces(&self) -> Result<Vec<ComputerId>> {
        self.traces.passive_traces().await
    }

    pub async fn strikes(&self) -> Result<u8> {
        self.traces.strikes().await
    }

    pub async fn is_trace_protected(&self, remote: ComputerId) -> Result<bool> {
        let Some(row) = self.computers.computer(remote).await? else {
            return Ok(false);
        };
        let Some((owner, _)) = self.guilds.company_majority_owner(row.network).await? else {
            return Ok(false);
        };

        if self
            .guilds
            .guilds
            .guild_of_account(&owner)
            .await?
            .is_some_and(|guild| guild.player_member)
        {
            return Ok(true);
        }

        let account = match self.npcs.person_on(self.world.player()).await? {
            Some(person) => self.bank.bank_number_of(person).await?.unwrap_or_default(),
            None => String::new(),
        };

        Ok(owner == account)
    }

    pub async fn trace_admin(&self, remote: ComputerId) -> Result<Option<(Npc, u32, NetworkKind)>> {
        let Some(row) = self.computers.computer(remote).await? else {
            return Ok(None);
        };

        if row.network_kind.is_player_made() {
            return Ok(None);
        }

        let (Some(person), Some(admin)) = (
            self.npcs.network_admin_person(row.network).await?,
            self.npcs.network_admin(row.network).await?,
        ) else {
            return Ok(None);
        };

        Ok(self
            .npcs
            .npc(person)
            .await?
            .map(|npc| (npc, u32::from(admin.pc_experience), row.network_kind)))
    }

    pub async fn trace_action(
        &self,
        remote: ComputerId,
        bounces: u32,
        action: LogAction,
        terminal: u64,
    ) -> Result<bool> {
        let Some((admin, _, _)) = self.trace_admin(remote).await? else {
            return Ok(false);
        };

        if !admin.online || admin.is_dead_or_fired(self.world.time_seconds()) {
            return Ok(false);
        }

        self.start_active_trace(remote, bounces, action, terminal)
            .await
    }

    pub async fn trace_connection(&self, remote: ComputerId) -> Result<bool> {
        let Some(row) = self.computers.computer(remote).await? else {
            return Ok(false);
        };

        if row.network_kind.is_player_made() {
            return Ok(false);
        }

        self.add_passive_trace(remote).await
    }

    pub async fn start_active_trace(
        &self,
        remote: ComputerId,
        bounces: u32,
        action: LogAction,
        terminal: u64,
    ) -> Result<bool> {
        if self.is_trace_protected(remote).await? {
            return Ok(false);
        }

        let Some((admin, exp_pcs, kind)) = self.trace_admin(remote).await? else {
            return Ok(false);
        };

        if admin.is_dead_or_fired(self.world.time_seconds())
            || !should_start_tracking(kind, exp_pcs, action)
        {
            return Ok(false);
        }

        if self.traces.active_traces().await?.iter().any(|trace| {
            trace.terminal == terminal && trace.admin == admin.person && trace.computer == remote
        }) {
            return Ok(false);
        }

        let ends_at = active_trace_end(self.world.time_seconds(), bounces, exp_pcs);
        let trace = self
            .traces
            .insert_active_trace(remote, admin.person, terminal, ends_at.unwrap_or(u64::MAX))
            .await?;

        if let Some(ends_at) = ends_at {
            crate::ActiveTrace::schedule_with(&self.world, &self.scheduler, ends_at, trace).await?;
        }

        Ok(true)
    }

    pub async fn cancel_active_traces(&self, terminals: &[u64]) -> Result<()> {
        for trace in self.traces.active_traces().await? {
            if !terminals.contains(&trace.terminal) {
                continue;
            }

            self.remove_active_trace(&trace).await?;
            self.add_passive_trace(trace.computer).await?;
        }

        Ok(())
    }

    pub async fn remove_active_trace(&self, trace: &ActiveTrace) -> Result<()> {
        if let Some(routine) = self.traces.active_trace_routine(trace.id).await? {
            self.scheduler
                .cancel(&self.world.db().connection(), routine)
                .await?;
        }

        self.traces.remove_active_trace(trace.id).await
    }

    pub async fn run_active_trace(&self, trace: crate::TraceId) -> Result<()> {
        let Some(trace) = self.traces.active_trace(trace).await? else {
            return Ok(());
        };

        self.traces.remove_active_trace(trace.id).await?;
        self.stop_passive_traces().await?;

        self.game_over(trace.computer, TraceKind::Active).await
    }

    pub async fn set_strikes(&self, strikes: u8) -> Result<()> {
        self.traces.set_strikes(strikes).await
    }

    pub async fn strike_player(&self, remote: ComputerId, trace: TraceKind) -> Result<()> {
        self.game_over(remote, trace).await
    }

    pub async fn game_over(&self, remote: ComputerId, trace: TraceKind) -> Result<()> {
        let strikes = add_strike(self.traces.strikes().await?);

        self.traces.set_strikes(strikes.count).await?;
        self.world.events().publish(StrikeEvent {
            computer: remote,
            trace,
            strikes: strikes.visible,
        });

        Ok(())
    }
}

/// A world service with only its declared dependencies.
pub struct Traces {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) traces: std::rc::Rc<crate::storage::TraceRepository>,
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_core::Dep<greyhack_world_core::FileSystems>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    pub(crate) guilds: greyhack_world_core::Dep<crate::GuildService>,
    pub(crate) npcs: greyhack_world_core::Dep<greyhack_world_npcs::NpcService>,
    pub(crate) bank: greyhack_world_core::Dep<greyhack_world_economy::Bank>,
    pub(crate) scheduler: std::rc::Rc<greyhack_world_routines::Scheduler>,
}

impl Traces {
    pub(crate) fn new(
        registry: &std::rc::Rc<greyhack_world_modules::Registry>,
        traces: std::rc::Rc<crate::storage::TraceRepository>,
    ) -> Self {
        Self {
            world: registry.world::<greyhack_world_core::WorldRef>().clone(),
            traces,
            computers: registry.get(),
            files: registry.get(),
            networks: registry.get(),
            guilds: registry.get(),
            npcs: registry.get(),
            bank: registry.get(),
            scheduler: registry.scheduler(),
        }
    }
}
