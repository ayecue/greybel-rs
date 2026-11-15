//! What scripts do to the processes of a computer through `ComputerIntrinsics`: listing and closing
//! them.

use greyhack_behaviour::{PROTECTED_PIDS, ROOT_USER};

use crate::systems::ProcessService as ProcessesService;
use crate::systems::computer::failure::ComputerFailure;
use crate::{ComputerId, Result};

/// Memory and processor use of one process (`ps`).
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessUsage {
    /// Owner of the process.
    pub user: String,
    /// Process ID.
    pub pid: u32,
    /// Processor use.
    pub cpu: f32,
    /// Share of the computer's memory in percent.
    pub memory: f32,
    /// Process name.
    pub name: String,
}

/// What a script's `close_program` did (`ComputerIntrinsics.close_program`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessClose {
    /// The computer no longer exists.
    ComputerMissing,
    /// The computer has no such process.
    Missing,
    /// The process ended.
    Success,
    /// The process is the command a terminal runs, which closes its own process.
    OwnProcess,
    /// The request is refusal.
    Refusal(ComputerFailure),
}

impl ProcessesService {
    /// Lists the processes of `computer` with their memory share (`ComputerIntrinsics.show_procs`),
    /// or `None` if it no longer exists. NPC user processes are intentionally omitted.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn process_usage(&self, computer: ComputerId) -> Result<Option<Vec<ProcessUsage>>> {
        if self.computer_rows.computer(computer).await?.is_none() {
            return Ok(None);
        }

        let ram = self.hardware_rows.hardware(computer).await?.ram_mb() as f32;

        Ok(Some(
            self.process_table(computer)
                .await?
                .into_iter()
                .map(|process| ProcessUsage {
                    user: process.user,
                    pid: process.pid,
                    cpu: process.cpu,
                    memory: process.ram_mb * 100.0 / ram,
                    name: process.name,
                })
                .collect(),
        ))
    }

    /// Terminates process `pid` as `user` and closes service-daemon ports
    /// (`ComputerIntrinsics.close_program`). `script_process` is the caller's root process and
    /// `terminal_command` the caller's own process when it is a command a terminal runs, which stops
    /// the caller instead.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn close_process(
        &self,
        computer: ComputerId,
        user: &str,
        pid: i32,
        script_process: Option<(ComputerId, u32)>,
        terminal_command: Option<(ComputerId, u32)>,
    ) -> Result<ProcessClose> {
        if self.computer_rows.computer(computer).await?.is_none() {
            return Ok(ProcessClose::Missing);
        }

        if pid >= PROTECTED_PIDS {
            return Ok(ProcessClose::Refusal(ComputerFailure::SystemProcess));
        }

        let Some(process) = (match u32::try_from(pid) {
            Ok(pid) => self.process(computer, pid).await?,
            Err(_) => None,
        }) else {
            return Ok(ProcessClose::Missing);
        };

        if user != ROOT_USER && process.user != user {
            return Ok(ProcessClose::Refusal(ComputerFailure::ForeignProcess {
                pid,
                user: process.user,
            }));
        }

        if process.protected {
            return Ok(ProcessClose::Refusal(ComputerFailure::SystemProcess));
        }

        let target = Some((computer, process.pid));

        // On the player's computer, a script can't close the process of the program its terminal
        // started (`ComputerIntrinsics.close_program`, `GreyInterpreter.PID`).
        if self.world.is_player(computer) && script_process == target {
            return Ok(ProcessClose::Refusal(ComputerFailure::ScriptProcess));
        }

        // A command a terminal runs closing its own process stops (`HelperScript.subPID`); the
        // runtime's end of the program then removes the process.
        if terminal_command == target {
            return Ok(ProcessClose::OwnProcess);
        }

        self.end_process(computer, process.pid).await?;

        if let Some(service) = process.service {
            self.stop_service(computer, service).await?;
        }

        Ok(ProcessClose::Success)
    }
}
