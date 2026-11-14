//! Passive traces: the follow-up of a finished program's trace back along the bounces to the player
//! (`PassiveTrace`).

use greyhack_behaviour::{
    FileKind, LogAction, MAX_PASSIVE_TRACES, PASSIVE_BOUNCE_MS, PASSIVE_NEXT_ADMIN_MS,
    PassiveStage, SYSTEM_LOG_PATH, SystemLog, passive_trace_delay_ms, requeues_absent_admin,
    should_start_tracking, world_seconds,
};

use crate::{ComputerId, PassiveState, PassiveTrace, Result, TraceKind, Traces};

/// Methods of [`World`] for guilds passive traces.
impl Traces {
    pub async fn add_passive_trace(&self, remote: ComputerId) -> Result<bool> {
        if !self.computers.has_computer(remote).await? || self.is_trace_protected(remote).await? {
            return Ok(false);
        }

        let queue = self.traces.passive_traces().await?;

        if queue.len() >= MAX_PASSIVE_TRACES || queue.contains(&remote) {
            return Ok(false);
        }

        self.traces.push_passive_trace(remote).await?;

        let state = self.traces.passive_state().await?;

        if !state.running {
            self.start_passive_trace_loop().await?;
        }

        Ok(true)
    }

    pub async fn stop_passive_traces(&self) -> Result<()> {
        PassiveTrace::stop_with(&self.world, &self.scheduler).await?;

        self.reset_passive_traces().await
    }

    pub async fn run_passive_trace(&self) -> Result<()> {
        let state = self.traces.passive_state().await?;

        if !state.running {
            return Ok(());
        }

        match state.stage {
            PassiveStage::Wait => self.look_at_passive_head().await,
            PassiveStage::Read => self.read_passive_log().await,
            PassiveStage::Bounce => self.bounce_passive_trace(&state).await,
        }
    }
}

impl Traces {
    pub async fn reset_passive_traces(&self) -> Result<()> {
        self.traces.clear_passive_traces().await?;

        self.traces
            .set_passive_state(&PassiveState::default())
            .await
    }

    pub async fn start_passive_trace_loop(&self) -> Result<()> {
        self.traces
            .set_passive_state(&PassiveState {
                running: true,
                stage: PassiveStage::Wait,
                ..PassiveState::default()
            })
            .await?;
        PassiveTrace::schedule_with(
            &self.world,
            &self.scheduler,
            self.world.time_seconds() + world_seconds(PASSIVE_NEXT_ADMIN_MS),
        )
        .await?;

        Ok(())
    }

    pub async fn next_in_passive_queue(&self) -> Result<()> {
        if self.traces.passive_traces().await?.is_empty() {
            return self.reset_passive_traces().await;
        }

        self.start_passive_trace_loop().await
    }

    pub async fn look_at_passive_head(&self) -> Result<()> {
        let Some(head) = self.traces.passive_traces().await?.first().copied() else {
            return self.reset_passive_traces().await;
        };
        let admin = self.trace_admin(head).await?;
        let Some((_, exp_pcs, _)) = admin.filter(|(admin, _, _)| {
            admin.online && !admin.is_dead_or_fired(self.world.time_seconds())
        }) else {
            // The administrator is absent: the computer goes to the back of the queue.
            self.traces.remove_passive_trace(head).await?;

            if requeues_absent_admin(self.traces.passive_traces().await?.len()) {
                self.traces.push_passive_trace(head).await?;
            }

            return self.next_in_passive_queue().await;
        };

        if self.system_log_of(head).await?.is_none() {
            return self.finish_passive_trace(head, false).await;
        }

        self.traces
            .set_passive_state(&PassiveState {
                running: true,
                stage: PassiveStage::Read,
                ..PassiveState::default()
            })
            .await?;
        PassiveTrace::schedule_with(
            &self.world,
            &self.scheduler,
            self.world.time_seconds() + world_seconds(passive_trace_delay_ms(exp_pcs)),
        )
        .await?;

        Ok(())
    }

    pub async fn read_passive_log(&self) -> Result<()> {
        let Some(head) = self.traces.passive_traces().await?.first().copied() else {
            return self.reset_passive_traces().await;
        };
        let Some(log) = self.system_log_of(head).await? else {
            return self.finish_passive_trace(head, false).await;
        };
        let token = self.player_token_trace().await?;
        let player = self.computers.logs.player_net_id();
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
        let from_ip = self.networks.topology.public_ip(head).await?;

        self.traces
            .set_passive_state(&PassiveState {
                running: true,
                stage: PassiveStage::Bounce,
                from_ip,
                to_ip: entry.ip.clone(),
                origin: Some(head),
            })
            .await?;
        PassiveTrace::schedule_with(
            &self.world,
            &self.scheduler,
            self.world.time_seconds() + world_seconds(PASSIVE_BOUNCE_MS),
        )
        .await?;

        Ok(())
    }

    pub async fn bounce_passive_trace(&self, state: &PassiveState) -> Result<()> {
        let Some(head) = self.traces.passive_traces().await?.first().copied() else {
            return self.reset_passive_traces().await;
        };
        let Some(router) = self.networks.router_at(&state.to_ip).await? else {
            return self.finish_passive_trace(head, false).await;
        };
        let Some(log) = self.system_log_of(router.id.computer()).await? else {
            return self.finish_passive_trace(head, false).await;
        };
        let token = self.player_token_trace().await?;
        let player = self.computers.logs.player_net_id();
        let Some(entry) = log.traceable_bounce(&state.from_ip, &player, &token) else {
            return self.finish_passive_trace(head, false).await;
        };
        let lan = self
            .networks
            .topology
            .device(router.network, &entry.ip)
            .await?;

        if lan == Some(self.world.player()) {
            self.traces
                .set_token_generation(self.traces.token_generation().await? + 1)
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

        self.traces
            .set_passive_state(&PassiveState {
                running: true,
                stage: PassiveStage::Bounce,
                from_ip: state.to_ip.clone(),
                to_ip: next.ip.clone(),
                origin: state.origin,
            })
            .await?;
        PassiveTrace::schedule_with(
            &self.world,
            &self.scheduler,
            self.world.time_seconds() + world_seconds(PASSIVE_BOUNCE_MS),
        )
        .await?;

        Ok(())
    }

    pub async fn finish_passive_trace(&self, head: ComputerId, finished: bool) -> Result<()> {
        if finished {
            return self.reset_passive_traces().await;
        }

        self.traces.remove_passive_trace(head).await?;

        self.next_in_passive_queue().await
    }

    pub async fn system_log_of(&self, computer: ComputerId) -> Result<Option<SystemLog>> {
        let Some(file) = self
            .files
            .lookup
            .file(computer, SYSTEM_LOG_PATH, false)
            .await?
        else {
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
