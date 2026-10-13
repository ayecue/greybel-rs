//! Computer processes and RAM backed by database rows.

use greybel_source_map::SourceMap;
use greyhack_behaviour::{
    BASH_PATH, FileKind, MAX_TERMINALS, PROCESS_RAM_MB, SAFE_MODE_TERMINAL, SESSION_PROCESS,
    ServiceId, session_pid,
};
use greyhack_world_assets::Script;

use crate::{ComputerId, Process, ProcessFailure, Result, World};

impl World {
    /// Checks RAM for `additional_ram` MB and terminal capacity when `is_terminal`. Returns the
    /// resource failure if unavailable.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn check_process_capacity(
        &self,
        computer: ComputerId,
        additional_ram: f32,
        is_terminal: bool,
    ) -> Result<Result<(), ProcessFailure>> {
        let Some(row) = self.db.computer(computer).await? else {
            return Ok(Err(ProcessFailure::ComputerUnavailable));
        };
        let processes = self.db.processes(computer).await?;

        if is_terminal
            && processes
                .iter()
                .filter(|process| process.is_terminal)
                .count()
                >= MAX_TERMINALS
        {
            return Ok(Err(ProcessFailure::TooManyTerminals));
        }

        let used_ram: f32 = processes.iter().map(|process| process.ram_mb).sum();

        if used_ram + additional_ram > self.db.hardware(row.id).await?.ram_mb() as f32 {
            return Ok(Err(ProcessFailure::NoRam {
                remote: !self.is_player(computer),
            }));
        }

        Ok(Ok(()))
    }

    /// Returns `computer` processes ordered by PID.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn processes(&self, computer: ComputerId) -> Result<Vec<Process>> {
        self.db.processes(computer).await
    }

    /// Returns processes shown by `ps`: one `dsession` per login in login order, then processes by
    /// PID (`ComputerIntrinsics` `show_procs`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn process_table(&self, computer: ComputerId) -> Result<Vec<Process>> {
        let mut users: Vec<String> = Vec::new();

        for user in self.db.sessions(computer).await? {
            if !users.contains(&user) {
                users.push(user);
            }
        }

        let mut table = Vec::with_capacity(users.len());

        for user in users {
            let surname = match self
                .db
                .user(computer, &user)
                .await?
                .and_then(|user| user.person)
            {
                Some(person) => self.db.person(person).await?.map(|person| person.surname),
                None => None,
            };

            table.push(Process {
                pid: session_pid(&surname.unwrap_or_default()),
                user,
                name: SESSION_PROCESS.to_owned(),
                cpu: 0.0,
                ram_mb: PROCESS_RAM_MB,
                protected: false,
                service: None,
                is_terminal: false,
            });
        }

        table.extend(self.db.processes(computer).await?);

        Ok(table)
    }

    /// Returns whether `computer` runs the safe-mode terminal that [`Self::reboot_player`] starts
    /// (`PlayerHelperServer.CheckIniErrorServerRpc`). Its programs run in that terminal
    /// (`GreyScriptHelperServer.PrepareTerminalServerRpc`, `GreyInterpreter.isSafeTerminal`): the
    /// desktop is not running until the computer restarts normally.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn in_safe_mode(&self, computer: ComputerId) -> Result<bool> {
        Ok(self.db.processes(computer).await?.iter().any(|process| {
            process.protected && process.is_terminal && process.name == SAFE_MODE_TERMINAL
        }))
    }

    /// Returns `computer` process `pid`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn process(&self, computer: ComputerId, pid: u32) -> Result<Option<Process>> {
        self.db.process(computer, pid).await
    }

    /// Runs `command` as `user` on `computer` and returns its PID.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn spawn_process(
        &mut self,
        computer: ComputerId,
        user: &str,
        command: &str,
        service: Option<ServiceId>,
    ) -> Result<u32> {
        self.db
            .spawn_process(computer, user, command, service, false)
            .await
    }

    /// Starts a terminal process subject to the terminal limit and returns its PID.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn spawn_terminal(
        &mut self,
        computer: ComputerId,
        user: &str,
        command: &str,
    ) -> Result<u32> {
        self.db
            .spawn_process(computer, user, command, None, true)
            .await
    }

    /// Returns code and line source map for `start_terminal` (`Computer.GetExecutableBash`). Uses
    /// executable `/bin/bash` code when present, otherwise Grey Hack's built-in bash.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn bash_program(
        &self,
        computer: ComputerId,
        user: &str,
    ) -> Result<(String, SourceMap)> {
        let account = self.user(computer, user).await?;
        let program = match self.file(computer, BASH_PATH, true).await? {
            Some(node)
                if self
                    .has_node_permission(
                        &node,
                        greyhack_behaviour::Permission::Execute,
                        account.as_ref(),
                    )
                    .await? =>
            {
                node.data()
                    .filter(|data| data.kind == FileKind::Program && !data.content.is_empty())
                    .map(|data| (data.content.clone(), data.source_map.clone()))
            }
            _ => None,
        };

        Ok(program.unwrap_or_else(|| {
            let bash = Script::find("bash").expect("the game ships its bash");

            (bash.source.to_owned(), SourceMap::default())
        }))
    }

    /// Ends `computer` process `pid` and returns whether it was running.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn end_process(&mut self, computer: ComputerId, pid: u32) -> Result<bool> {
        self.process_ends.insert(computer);

        self.db.end_process(computer, pid).await
    }
}
