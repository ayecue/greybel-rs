//! What the world does when a script program ends (`GreyScriptHelperServer.PostEndScript`,
//! `HelperScript.CloseRemotePrograms`).

use greyhack_behaviour::LogAction;

use crate::{ComputerId, LogWrite, Result, World};

impl World {
    /// Ends the process of `program` and the traces the program caused. Both are attempted; the
    /// trace error wins when both fail.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn end_program_effects(
        &mut self,
        process: Option<(ComputerId, u32)>,
        program: u64,
    ) -> Result<()> {
        let process_end = match process {
            Some((computer, pid)) => self.end_process(computer, pid).await.map(|_| ()),
            None => Ok(()),
        };

        self.cancel_active_traces(&[program])
            .await
            .map(|_| ())
            .and(process_end)
    }

    /// Ends each of `processes` that is on an existing computer, when the connection of their
    /// programs closes.
    ///
    /// # Errors
    ///
    /// Returns the last database error after attempting every process.
    pub async fn end_orphaned_processes(&mut self, processes: &[(ComputerId, u32)]) -> Result<()> {
        let mut result = Ok(());

        for &(computer, pid) in processes {
            let outcome = match self.computer(computer).await {
                Ok(Some(_)) => self.end_process(computer, pid).await.map(|_| ()),
                Ok(None) => Ok(()),
                Err(error) => Err(error),
            };

            if outcome.is_err() {
                result = outcome;
            }
        }

        result
    }

    /// Returns whether the terminal on `host` still runs: its computer exists, and so does the root
    /// process of its tree.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_terminal_running(
        &self,
        host: ComputerId,
        root: (ComputerId, u32),
    ) -> Result<bool> {
        Ok(self.computer(host).await?.is_some() && self.process(root.0, root.1).await?.is_some())
    }

    /// Logs the exit of a terminal on `remote` that `source` ran from, as `MainClass` does when the
    /// terminal's bash ends with `exit`. Nothing happens when either computer is gone.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn log_terminal_exit(
        &mut self,
        source: ComputerId,
        remote: ComputerId,
    ) -> Result<()> {
        if self.computer(source).await?.is_none() || self.computer(remote).await?.is_none() {
            return Ok(());
        }

        let source = self.public_ip(source).await?;

        self.write_log(
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
