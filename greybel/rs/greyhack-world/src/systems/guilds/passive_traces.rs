//! Passive traces: the follow-up of a finished program's trace back along the bounces to the player
//! (`PassiveTrace`).

use greyhack_behaviour::{
    FileKind, LogAction, MAX_PASSIVE_TRACES, PASSIVE_BOUNCE_MS, PASSIVE_NEXT_ADMIN_MS,
    PassiveStage, SYSTEM_LOG_PATH, SystemLog, passive_trace_delay_ms, requeues_absent_admin,
    should_start_tracking, world_seconds,
};

use crate::{ComputerId, PassiveState, Result, Routine, TraceKind, World};

impl World {
    /// Queues `remote` for the passive trace unless the player owns its company, the queue is full
    /// or it contains `remote` already, and starts the trace when none runs
    /// (`PassiveTrace.AddPassiveTrace`). Returns whether it was queued.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn add_passive_trace(&mut self, remote: ComputerId) -> Result<bool> {
        if !self.db.has_computer(remote).await? || self.is_trace_protected(remote).await? {
            return Ok(false);
        }

        let queue = self.db.passive_traces().await?;

        if queue.len() >= MAX_PASSIVE_TRACES || queue.contains(&remote) {
            return Ok(false);
        }

        self.db.push_passive_trace(remote).await?;

        let state = self.db.passive_state().await?;

        if !state.running {
            self.start_passive_trace_loop().await?;
        }

        Ok(true)
    }

    /// Cancels the passive trace and forgets its queue (`PassiveTrace.StopTraces`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn stop_passive_traces(&mut self) -> Result<()> {
        for entry in self.scheduled_routines().await? {
            if entry.routine == Routine::PassiveTrace {
                self.db.remove_routine(entry.id).await?;
            }
        }

        self.reset_passive_traces().await
    }

    /// Empties the queue and stops the trace (`PassiveTrace.ResetTraces`).
    async fn reset_passive_traces(&mut self) -> Result<()> {
        self.db.clear_passive_traces().await?;

        self.db.set_passive_state(&PassiveState::default()).await
    }

    /// Starts waiting for the first computer of the queue (`PassiveTrace.ManageTraces`).
    async fn start_passive_trace_loop(&mut self) -> Result<()> {
        self.db
            .set_passive_state(&PassiveState {
                running: true,
                stage: PassiveStage::Wait,
                ..PassiveState::default()
            })
            .await?;
        self.schedule(
            self.time_seconds + world_seconds(PASSIVE_NEXT_ADMIN_MS),
            Routine::PassiveTrace,
        )
        .await?;

        Ok(())
    }

    /// Goes on with the next computer of the queue, or ends the trace when it is empty.
    async fn next_in_passive_queue(&mut self) -> Result<()> {
        if self.db.passive_traces().await?.is_empty() {
            return self.reset_passive_traces().await;
        }

        self.start_passive_trace_loop().await
    }

    /// Runs the step of the passive trace that is due.
    pub(crate) async fn run_passive_trace(&mut self) -> Result<()> {
        let state = self.db.passive_state().await?;

        if !state.running {
            return Ok(());
        }

        match state.stage {
            PassiveStage::Wait => self.look_at_passive_head().await,
            PassiveStage::Read => self.read_passive_log().await,
            PassiveStage::Bounce => self.bounce_passive_trace(&state).await,
        }
    }

    /// Takes the first computer of the queue once its administrator is at work and waits for the
    /// administrator to trace the connection (`PassiveTrace.ManageTraces`,
    /// `PassiveTrace.StartPassiveTrace`).
    async fn look_at_passive_head(&mut self) -> Result<()> {
        let Some(head) = self.db.passive_traces().await?.first().copied() else {
            return self.reset_passive_traces().await;
        };
        let admin = self.trace_admin(head).await?;
        let Some((_, exp_pcs, _)) = admin
            .filter(|(admin, _, _)| admin.online && !admin.is_dead_or_fired(self.time_seconds))
        else {
            // The administrator is absent: the computer goes to the back of the queue.
            self.db.remove_passive_trace(head).await?;

            if requeues_absent_admin(self.db.passive_traces().await?.len()) {
                self.db.push_passive_trace(head).await?;
            }

            return self.next_in_passive_queue().await;
        };

        if self.system_log_of(head).await?.is_none() {
            return self.finish_passive_trace(head, false).await;
        }

        self.db
            .set_passive_state(&PassiveState {
                running: true,
                stage: PassiveStage::Read,
                ..PassiveState::default()
            })
            .await?;
        self.schedule(
            self.time_seconds + world_seconds(passive_trace_delay_ms(exp_pcs)),
            Routine::PassiveTrace,
        )
        .await?;

        Ok(())
    }

    /// Finds the latest entry the player's actions left in the computer's log that starts a trace
    /// there and follows the connection back from it (`PassiveTrace.StartPassiveTrace`).
    async fn read_passive_log(&mut self) -> Result<()> {
        let Some(head) = self.db.passive_traces().await?.first().copied() else {
            return self.reset_passive_traces().await;
        };
        let Some(log) = self.system_log_of(head).await? else {
            return self.finish_passive_trace(head, false).await;
        };
        let token = self.player_token_trace().await?;
        let player = self.player_net_id();
        let Some((_, exp_pcs, kind)) = self.trace_admin(head).await? else {
            return self.finish_passive_trace(head, false).await;
        };
        let only_close =
            log.content_log.len() == 1 && log.content_log[0].action == LogAction::Close;
        let Some(entry) = log.content_log.iter().rev().find(|entry| {
            !entry.tutorial
                && (only_close || should_start_tracking(kind, exp_pcs, entry.action))
                && entry.player_net_id == player
                && entry.token_trace == token
        }) else {
            return self.finish_passive_trace(head, false).await;
        };
        let from_ip = self.public_ip(head).await?;

        self.db
            .set_passive_state(&PassiveState {
                running: true,
                stage: PassiveStage::Bounce,
                from_ip,
                to_ip: entry.ip.clone(),
                origin: Some(head),
            })
            .await?;
        self.schedule(
            self.time_seconds + world_seconds(PASSIVE_BOUNCE_MS),
            Routine::PassiveTrace,
        )
        .await?;

        Ok(())
    }

    /// Follows the connection one hop back (`PassiveTrace.BounceTrace`).
    async fn bounce_passive_trace(&mut self, state: &PassiveState) -> Result<()> {
        let Some(head) = self.db.passive_traces().await?.first().copied() else {
            return self.reset_passive_traces().await;
        };
        let Some(router) = self.db.router_at(&state.to_ip).await? else {
            return self.finish_passive_trace(head, false).await;
        };
        let Some(log) = self.system_log_of(router.id.computer()).await? else {
            return self.finish_passive_trace(head, false).await;
        };
        let token = self.player_token_trace().await?;
        let player = self.player_net_id();
        let Some(entry) = log.traceable_bounce(&state.from_ip, &player, &token) else {
            return self.finish_passive_trace(head, false).await;
        };
        let lan = self.device(router.network, &entry.ip).await?;

        if lan == Some(self.player) {
            self.db
                .set_token_generation(self.db.token_generation().await? + 1)
                .await?;

            if let Some(origin) = state.origin {
                self.game_over(origin, TraceKind::Passive).await?;
            }

            return self.finish_passive_trace(head, true).await;
        }

        let Some(lan) = lan else {
            return self.finish_passive_trace(head, false).await;
        };
        let Some(lan_log) = self.system_log_of(lan).await? else {
            return self.finish_passive_trace(head, false).await;
        };
        let Some(next) = lan_log.traceable_bounce(&state.from_ip, &player, &token) else {
            return self.finish_passive_trace(head, true).await;
        };

        self.db
            .set_passive_state(&PassiveState {
                running: true,
                stage: PassiveStage::Bounce,
                from_ip: state.to_ip.clone(),
                to_ip: next.ip.clone(),
                origin: state.origin,
            })
            .await?;
        self.schedule(
            self.time_seconds + world_seconds(PASSIVE_BOUNCE_MS),
            Routine::PassiveTrace,
        )
        .await?;

        Ok(())
    }

    /// Ends the trace of the computer at the head of the queue: a finished trace empties the queue,
    /// and any other moves on to the next computer (`PassiveTrace.ManageTraces`).
    async fn finish_passive_trace(&mut self, head: ComputerId, finished: bool) -> Result<()> {
        if finished {
            return self.reset_passive_traces().await;
        }

        self.db.remove_passive_trace(head).await?;

        self.next_in_passive_queue().await
    }

    /// Reads the system log of `computer`, `None` when it has none, it is no log or it is empty
    /// (`FileSystem.TypeFile.SystemLog`).
    async fn system_log_of(&self, computer: ComputerId) -> Result<Option<SystemLog>> {
        let Some(file) = self.file(computer, SYSTEM_LOG_PATH, false).await? else {
            return Ok(None);
        };
        let Some(data) = file.data() else {
            return Ok(None);
        };

        if data.kind != FileKind::SystemLog || data.content.is_empty() {
            return Ok(None);
        }

        Ok(SystemLog::parse(&data.content))
    }
}

#[cfg(test)]
#[path = "tests/passive_traces.rs"]
mod tests;
