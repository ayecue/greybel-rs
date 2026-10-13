//! Active traces: administrators tracing the player while they act on a company's computers, and
//! the strikes that follow (`ActiveTraceSystem`, `PlayerHelperServer.ActiveGameOver`).
//!
//! Grey Hack runs traces in real time; the world runs them in world time, one real second being
//! [`greyhack_behaviour::TIME_SCALE`] world seconds.

use greyhack_behaviour::{
    LogAction, NetworkKind, active_trace_end, add_strike, should_start_tracking, token_trace,
};

use crate::{ActiveTrace, ComputerId, Npc, Result, Routine, TraceKind, World, WorldEvent};

impl World {
    /// Returns the token the player's log entries carry now.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn player_token_trace(&self) -> Result<String> {
        Ok(token_trace(self.seed, self.db.token_generation().await?))
    }

    /// Returns the active traces in the order they started.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn active_traces(&self) -> Result<Vec<ActiveTrace>> {
        self.db.active_traces().await
    }

    /// Returns the computers the passive trace goes through, in order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn passive_traces(&self) -> Result<Vec<ComputerId>> {
        self.db.passive_traces().await
    }

    /// Returns the player's strikes (`PlayerHelperServer.gameOverCont`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn strikes(&self) -> Result<u8> {
        self.db.strikes().await
    }

    /// Returns whether the player owns `remote`'s company, as a majority shareholder or as a member
    /// of the guild that does, so no administrator traces them there
    /// (`ActiveTraceSystem.IsTraceProtectedByCompanyOwnership`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_trace_protected(&self, remote: ComputerId) -> Result<bool> {
        let Some(row) = self.db.computer(remote).await? else {
            return Ok(false);
        };
        let Some((owner, _)) = self.company_majority_owner(row.network).await? else {
            return Ok(false);
        };

        if self
            .db
            .guild_of_account(&owner)
            .await?
            .is_some_and(|guild| guild.player_member)
        {
            return Ok(true);
        }

        let account = match self.person_on(self.player).await? {
            Some(person) => self.bank_number_of(person).await?.unwrap_or_default(),
            None => String::new(),
        };

        Ok(owner == account)
    }

    /// Returns the administrator of `remote`'s network, the PC experience they trace with and the
    /// kind of network `remote` was made for (`Computer.GetAdmin`); `None` on networks the player
    /// made and where nobody administers.
    pub(super) async fn trace_admin(
        &self,
        remote: ComputerId,
    ) -> Result<Option<(Npc, u32, NetworkKind)>> {
        let Some(row) = self.db.computer(remote).await? else {
            return Ok(None);
        };

        if row.network_kind.is_player_made() {
            return Ok(None);
        }

        let (Some(person), Some(admin)) = (
            self.db.network_admin_person(row.network).await?,
            self.db.network_admin(row.network).await?,
        ) else {
            return Ok(None);
        };

        Ok(self
            .db
            .npc(person)
            .await?
            .map(|npc| (npc, u32::from(admin.pc_experience), row.network_kind)))
    }

    /// Traces the player after `action` on `remote`, reached through `bounces` bounces by the program
    /// `terminal`, when the administrator is at work (`HelperScript.AddPendingActiveTrace`,
    /// `ActiveTraceSystem.StartActiveTrace`). Returns whether a trace started.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn trace_action(
        &mut self,
        remote: ComputerId,
        bounces: u32,
        action: LogAction,
        terminal: u64,
    ) -> Result<bool> {
        let Some((admin, _, _)) = self.trace_admin(remote).await? else {
            return Ok(false);
        };

        if !admin.online || admin.is_dead_or_fired(self.time_seconds) {
            return Ok(false);
        }

        self.start_active_trace(remote, bounces, action, terminal)
            .await
    }

    /// Queues `remote` for the passive trace after the player logged in to it, unless the player
    /// made its network (`PlayerUtils.ConnectService`, `GreyInterpreter.AddPendingPassiveTrace`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn trace_connection(&mut self, remote: ComputerId) -> Result<bool> {
        let Some(row) = self.db.computer(remote).await? else {
            return Ok(false);
        };

        if row.network_kind.is_player_made() {
            return Ok(false);
        }

        self.add_passive_trace(remote).await
    }

    /// Starts an active trace of the player on `remote` unless they own its company, its
    /// administrator is dead or fired, `action` does not start one there, or the program has one
    /// there already (`ActiveTraceSystem.StartActiveTrace`). Returns whether one started.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn start_active_trace(
        &mut self,
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

        if admin.is_dead_or_fired(self.time_seconds)
            || !should_start_tracking(kind, exp_pcs, action)
        {
            return Ok(false);
        }

        if self.db.active_traces().await?.iter().any(|trace| {
            trace.terminal == terminal && trace.admin == admin.person && trace.computer == remote
        }) {
            return Ok(false);
        }

        let ends_at = active_trace_end(self.time_seconds, bounces, exp_pcs);
        let trace = self
            .db
            .insert_active_trace(remote, admin.person, terminal, ends_at.unwrap_or(u64::MAX))
            .await?;

        if let Some(ends_at) = ends_at {
            self.schedule(ends_at, Routine::ActiveTrace { trace })
                .await?;
        }

        Ok(true)
    }

    /// Ends the active traces the programs `terminals` caused: each starts a passive trace of its
    /// computer instead (`ActiveTraceSystem.CancelAllActiveTraces`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn cancel_active_traces(&mut self, terminals: &[u64]) -> Result<()> {
        for trace in self.db.active_traces().await? {
            if !terminals.contains(&trace.terminal) {
                continue;
            }

            self.remove_active_trace(&trace).await?;
            self.add_passive_trace(trace.computer).await?;
        }

        Ok(())
    }

    /// Removes `trace` and its scheduled end.
    async fn remove_active_trace(&mut self, trace: &ActiveTrace) -> Result<()> {
        if let Some(routine) = self.db.active_trace_routine(trace.id).await? {
            self.db.remove_routine(routine).await?;
        }

        self.db.remove_active_trace(trace.id).await
    }

    /// An active trace ran out: the passive trace stops and the player gets a strike
    /// (`ActiveTraceSystem.ProgressActiveTrace`).
    pub(crate) async fn run_active_trace(&mut self, trace: crate::TraceId) -> Result<()> {
        let Some(trace) = self.db.active_trace(trace).await? else {
            return Ok(());
        };

        self.db.remove_active_trace(trace.id).await?;
        self.stop_passive_traces().await?;

        self.game_over(trace.computer, TraceKind::Active).await
    }

    /// Gives the player a strike for being traced to `remote` and reports it; the third ends the
    /// game (`PlayerHelperServer.ActiveGameOver`). Ending the game does not delete the player's
    /// computer in the world: the strikes start again.
    pub(super) async fn game_over(&mut self, remote: ComputerId, trace: TraceKind) -> Result<()> {
        let strikes = add_strike(self.db.strikes().await?);

        self.db.set_strikes(strikes.count).await?;
        self.events.push(WorldEvent::GameOver {
            computer: remote,
            trace,
            strikes: strikes.visible,
        });

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/active_traces.rs"]
pub(in crate::systems) mod tests;
