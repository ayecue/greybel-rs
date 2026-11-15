//! What the world does when a script program ends (`GreyScriptHelperServer.PostEndScript`,
//! `HelperScript.CloseRemotePrograms`).

use greyhack_behaviour::LogAction;
use greyhack_world_core::{ComputerId, LogWrite, Result};

/// Process, trace, and terminal-log effects of ending a program.
impl ShellService {
    pub async fn end_program_effects(
        &self,
        process: Option<(ComputerId, u32)>,
        program: u64,
    ) -> Result<()> {
        let process_end = match process {
            Some((computer, pid)) => self
                .computers
                .processes
                .end_process(computer, pid)
                .await
                .map(|_| ()),
            None => Ok(()),
        };

        self.traces
            .cancel_active_traces(&[program])
            .await
            .and(process_end)
    }

    pub async fn end_orphaned_processes(&self, processes: &[(ComputerId, u32)]) -> Result<()> {
        let mut result = Ok(());

        for &(computer, pid) in processes {
            let outcome = match self.networks.topology.computer(computer).await {
                Ok(Some(_)) => self
                    .computers
                    .processes
                    .end_process(computer, pid)
                    .await
                    .map(|_| ()),
                Ok(None) => Ok(()),
                Err(error) => Err(error),
            };

            if outcome.is_err() {
                result = outcome;
            }
        }

        result
    }

    pub async fn is_terminal_running(
        &self,
        host: ComputerId,
        root: (ComputerId, u32),
    ) -> Result<bool> {
        Ok(self.networks.topology.computer(host).await?.is_some()
            && self
                .computers
                .processes
                .process(root.0, root.1)
                .await?
                .is_some())
    }

    pub async fn log_terminal_exit(&self, source: ComputerId, remote: ComputerId) -> Result<()> {
        if self.networks.topology.computer(source).await?.is_none()
            || self.networks.topology.computer(remote).await?.is_none()
        {
            return Ok(());
        }

        let source = self.networks.topology.public_ip(source).await?;

        self.computers
            .logs
            .write_log(
                remote,
                &LogWrite {
                    action: LogAction::Close,
                    ip: &source,
                    port: 0,
                    file: "",
                    bounce_ip: "",
                },
            )
            .await
    }
}

/// A world service with only its declared dependencies.
pub struct ShellService {
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) traces: greyhack_world_modules::Dep<greyhack_world_traces::TraceService>,
}

impl ShellService {
    pub(crate) fn new(
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
        traces: greyhack_world_modules::Dep<greyhack_world_traces::TraceService>,
    ) -> Self {
        Self {
            computers,
            networks,
            traces,
        }
    }
}
