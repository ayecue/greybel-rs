//! Child programs a script starts: `launch` and `start_terminal` (`ShellIntrinsics`).

use greybel_source_map::SourceMap;
use greyhack_behaviour::{
    MAX_LAUNCH_DEPTH, PROCESS_RAM_MB, Permission, forbidden_param_token, path,
};

use crate::{ComputerId, ProcessFailure, Result};

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
    /// The computer no longer exists.
    Missing,
    /// The program does not start.
    Refusal(LaunchFailure),
    /// The program started.
    Program(LaunchedProgram),
}

/// Why a terminal does not start (`ShellIntrinsics.start_terminal`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalFailure {
    /// The computer no longer exists.
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

/// Child programs and terminal startup.
impl LaunchService {
    pub async fn launch_program(
        &self,
        computer: ComputerId,
        user: &str,
        path: &str,
        params: &[String],
        depth: u32,
        safe_terminal: bool,
    ) -> Result<Launch> {
        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(Launch::Missing);
        }

        let refusal = |failure| Ok(Launch::Refusal(failure));
        let Some(node) = self.files.lookup.file(computer, path, true).await? else {
            return refusal(LaunchFailure::NoCommand(path.to_owned()));
        };
        let Some(data) = node.data().filter(|data| data.is_executable()) else {
            return refusal(LaunchFailure::NotExecutable(node.name().to_owned()));
        };
        let account = self.computers.users.user(computer, user).await?;
        // A program of the safe-mode terminal doesn't need the file's execute permission on the player's
        // computer (`ShellIntrinsics.launch`); the access of its folders is still checked.
        let safe_terminal = safe_terminal && computer == self.world.player();

        if !self
            .files
            .permissions
            .can_access(&node, account.as_ref())
            .await?
            || (!safe_terminal
                && !self
                    .files
                    .permissions
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
            .computers
            .processes
            .check_process_capacity(computer, PROCESS_RAM_MB, false)
            .await?
        {
            return refusal(LaunchFailure::Process(failure));
        }

        let pid = self
            .computers
            .processes
            .spawn_process(computer, user, &name, None)
            .await?;

        Ok(Launch::Program(LaunchedProgram {
            process: (computer, pid),
            path: format!("{}/{name}", path::parent(path)),
            source,
            source_map,
        }))
    }

    pub async fn terminal_program(
        &self,
        computer: ComputerId,
        user: &str,
        depth: u32,
    ) -> Result<std::result::Result<TerminalProgram, TerminalFailure>> {
        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(Err(TerminalFailure::ComputerUnavailable));
        }

        if depth > MAX_LAUNCH_DEPTH {
            return Ok(Err(TerminalFailure::TooManyStackCalls));
        }

        let (source, source_map) = self
            .computers
            .processes
            .bash_program(computer, user)
            .await?;

        Ok(Ok(TerminalProgram { source, source_map }))
    }
}

/// A world service with only its declared dependencies.
pub struct LaunchService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_core::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
}

impl LaunchService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
        files: greyhack_world_core::Dep<greyhack_world_core::FileSystemService>,
        networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    ) -> Self {
        Self {
            world: world.clone(),
            computers,
            files,
            networks,
        }
    }
}
