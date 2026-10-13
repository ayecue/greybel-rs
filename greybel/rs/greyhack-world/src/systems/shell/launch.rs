//! Child programs a script starts: `launch` and `start_terminal` (`ShellIntrinsics`).

use greybel_source_map::SourceMap;
use greyhack_behaviour::{
    MAX_LAUNCH_DEPTH, PROCESS_RAM_MB, Permission, forbidden_param_token, path,
};

use crate::{ComputerId, ProcessFailure, Result, World};

/// Why a program does not start (`ShellIntrinsics.launch`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchFailure {
    /// No file is at the path.
    NoCommand(String),
    /// The file is not a program; contains its name.
    NotExecutable(String),
    /// The user cannot run the file.
    NoPermission,
    /// A parameter has a forbidden token.
    InvalidParameter(String),
    /// The program nesting exceeds the limit.
    TooManyStackCalls,
    /// The computer cannot run another process.
    Process(ProcessFailure),
}

/// A program that started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchedProgram {
    /// Computer and process ID of the program.
    pub process: (ComputerId, u32),
    /// Executable path with the process name.
    pub path: String,
    /// Source code.
    pub source: String,
    /// Source map of the code.
    pub source_map: SourceMap,
}

/// Result of a launch (`ShellIntrinsics.launch`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// The computer is gone.
    Gone,
    /// The program does not start.
    Refusal(LaunchFailure),
    /// The program started.
    Program(LaunchedProgram),
}

/// Why a terminal does not start (`ShellIntrinsics.start_terminal`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalFailure {
    /// The computer is gone.
    ComputerUnavailable,
    /// The program nesting exceeds the limit.
    TooManyStackCalls,
}

/// Code of a terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalProgram {
    /// Source code.
    pub source: String,
    /// Source map of the code.
    pub source_map: SourceMap,
}

impl World {
    /// Launches executable `path` as `user` on `computer` at nesting `depth`
    /// (`ShellIntrinsics.launch`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn launch_program(
        &mut self,
        computer: ComputerId,
        user: &str,
        path: &str,
        params: &[String],
        depth: u32,
    ) -> Result<Launch> {
        if self.computer(computer).await?.is_none() {
            return Ok(Launch::Gone);
        }

        let refusal = |failure| Ok(Launch::Refusal(failure));
        let Some(node) = self.file(computer, path, true).await? else {
            return refusal(LaunchFailure::NoCommand(path.to_owned()));
        };
        let Some(data) = node.data().filter(|data| data.is_executable()) else {
            return refusal(LaunchFailure::NotExecutable(node.name().to_owned()));
        };
        let account = self.user(computer, user).await?;
        // The safe-mode terminal of the player's computer doesn't need the file's execute permission
        // (`GreyInterpreter.isSafeTerminal`); the access of its folders is still checked.
        let safe_terminal = computer == self.player() && self.in_safe_mode(computer).await?;

        if !self.can_access(&node, account.as_ref()).await?
            || (!safe_terminal
                && !self
                    .has_node_permission(&node, Permission::Execute, account.as_ref())
                    .await?)
        {
            return refusal(LaunchFailure::NoPermission);
        }

        let name = if data.command_name.is_empty() {
            node.name().to_owned()
        } else {
            data.command_name.replace(".exe", "")
        };
        let (source, source_map) = (data.content.clone(), data.source_map.clone());

        // Validate parameters after executable checks and before compilation
        // (`ShellIntrinsics.launch`).
        if let Some(token) = params.iter().find_map(|param| forbidden_param_token(param)) {
            return refusal(LaunchFailure::InvalidParameter(token.to_owned()));
        }

        if depth > MAX_LAUNCH_DEPTH {
            return refusal(LaunchFailure::TooManyStackCalls);
        }

        if let Err(failure) = self
            .check_process_capacity(computer, PROCESS_RAM_MB, false)
            .await?
        {
            return refusal(LaunchFailure::Process(failure));
        }

        let pid = self.spawn_process(computer, user, &name, None).await?;

        Ok(Launch::Program(LaunchedProgram {
            process: (computer, pid),
            path: format!("{}/{name}", path::parent(path)),
            source,
            source_map,
        }))
    }

    /// Starts a terminal at nesting `depth` as `user` on `computer` (`ShellIntrinsics.start_terminal`)
    /// using the computer's `/bin/bash` program or the built-in game bash.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn terminal_program(
        &self,
        computer: ComputerId,
        user: &str,
        depth: u32,
    ) -> Result<std::result::Result<TerminalProgram, TerminalFailure>> {
        if self.computer(computer).await?.is_none() {
            return Ok(Err(TerminalFailure::ComputerUnavailable));
        }

        if depth > MAX_LAUNCH_DEPTH {
            return Ok(Err(TerminalFailure::TooManyStackCalls));
        }

        let (source, source_map) = self.bash_program(computer, user).await?;

        Ok(Ok(TerminalProgram { source, source_map }))
    }
}
