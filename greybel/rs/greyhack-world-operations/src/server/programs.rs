//! Programs owned by each connection, tracked until the runtime acknowledges termination. Ended program
//! IDs remain as tombstones until the connection closes so late operations cannot reach a reused ID.

use std::collections::HashMap;
use std::rc::Rc;

use greyhack_world::ComputerId;

use super::ConnectionChannel;
use crate::protocol::{EndReason, ObjectId, ProgramId};

/// Process key containing computer ID and PID.
pub(crate) type Process = (ComputerId, u32);

/// Connection channels and their program records.
#[derive(Default)]
pub(crate) struct ProgramTable {
    connections: HashMap<u32, ConnectionChannel>,
    programs: HashMap<u32, Program>,
    next_connection: u32,
    next_program: u32,
}

/// Program lifecycle stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// Running program that accepts operations.
    Active,
    /// Server-terminated program ([`EndReason`]) awaiting runtime acknowledgment.
    Termination,
    /// Runtime-ended program retaining only its ID to reject late operations.
    Tombstone,
}

/// Program process, connection, shell session, parent launch, tree root, and launch depth.
struct Program {
    /// Absent for `start_terminal` programs, which have no process, and launched programs whose process
    /// `start_terminal` closed.
    process: Option<Process>,
    connection: u32,
    session: ObjectId,
    parent: Option<u32>,
    /// Root runtime-started program for this launch tree.
    root: u32,
    depth: u32,
    stage: Stage,
    /// Terminal computers for a runtime-started program's tree (`HelperScript.pendingShell`).
    terminal_computers: Vec<ComputerId>,
    /// Whether `start_terminal` started it: the programs it launches are the commands of a terminal.
    terminal: bool,
    /// For a `start_terminal` program, where its terminal runs and how far away that is.
    terminal_info: Option<TerminalInfo>,
    /// For a runtime-started program, the computers its tree started terminals on, in order, until they exit
    /// (`Proceso.remoteMainTerminalPID` of the terminal process).
    remote_terminals: Vec<ComputerId>,
}

/// Where a `start_terminal` program runs and the bounces its exit compares (`HelperScript.CloseRemotePrograms`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalInfo {
    /// The shell's computer, where the terminal runs.
    pub(crate) host: ComputerId,
    /// Bounces of the terminal's shell (`MainClass`'s `startNumBounces` while the terminal ends).
    pub(crate) bounces: u32,
    /// Bounces of the program that started the terminal, which resumes when it ends.
    pub(crate) launcher_bounces: u32,
}

/// What a terminal's exit closes: the terminal on `remote`, started from `source` (`HelperScript.CloseRemotePrograms`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalClose {
    /// The computer the terminal's connection came from.
    pub(crate) source: ComputerId,
    /// The computer the terminal ran on.
    pub(crate) remote: ComputerId,
}

/// Runtime-started program and its descendants, which share one runtime heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Tree {
    pub(crate) connection: u32,
    pub(crate) root: u32,
}

/// Server-side program termination and its notification channel.
pub(crate) struct Termination {
    pub(crate) program: ProgramId,
    pub(crate) reason: EndReason,
    pub(crate) process: Option<Process>,
    pub(crate) connection: u32,
    pub(crate) channel: ConnectionChannel,
}

/// Result of handling a runtime's [`EndProgram`](crate::protocol::EndProgram).
pub(crate) struct ClientEnd {
    /// Running process that the caller must remove.
    pub(crate) process: Option<Process>,
    /// Running child programs terminated with the parent.
    pub(crate) children: Vec<Termination>,
}

impl ProgramTable {
    pub(crate) fn add_connection(&mut self, channel: ConnectionChannel) -> u32 {
        let id = self.next_connection;

        self.next_connection += 1;
        self.connections.insert(id, channel);

        id
    }

    /// Removes `connection` and all its programs, returning its channel and active processes. The caller
    /// drops the channel after releasing the table borrow to avoid recursive close handling during mutation.
    pub(crate) fn remove_connection(
        &mut self,
        connection: u32,
    ) -> (Option<ConnectionChannel>, Vec<Process>) {
        let channel = self.connections.remove(&connection);
        let mut processes = Vec::new();

        self.programs.retain(|_, program| {
            let owned = program.connection == connection;

            if owned && program.stage == Stage::Active {
                processes.extend(program.process);
            }

            !owned
        });

        (channel, processes)
    }

    /// Adds a running program of `connection` with `process`, shell `session`, and optional `parent`. Returns `None` after
    /// program IDs are exhausted.
    pub(crate) fn add_program(
        &mut self,
        connection: u32,
        process: Option<Process>,
        session: ObjectId,
        parent: Option<ProgramId>,
    ) -> Option<ProgramId> {
        let id = self.next_program;

        self.next_program = self.next_program.checked_add(1)?;

        let launcher = parent.and_then(|parent| self.programs.get(&parent.id));
        let depth = launcher.map_or(0, |launcher| launcher.depth + 1);
        let root = launcher.map_or(id, |launcher| launcher.root);

        self.programs.insert(
            id,
            Program {
                process,
                connection,
                session,
                parent: parent.map(|parent| parent.id),
                root,
                depth,
                stage: Stage::Active,
                terminal_computers: Vec::new(),
                terminal: false,
                terminal_info: None,
                remote_terminals: Vec::new(),
            },
        );

        Some(ProgramId { id })
    }

    /// Marks `program` as started by `start_terminal` with `info`.
    pub(crate) fn set_terminal(&mut self, program: ProgramId, info: TerminalInfo) {
        if let Some(entry) = self.programs.get_mut(&program.id) {
            entry.terminal = true;
            entry.terminal_info = Some(info);
        }
    }

    /// Adds `computer` to the terminals the tree of `connection`'s `program` started, which
    /// `ShellIntrinsics.start_terminal` does before it checks anything (`Proceso.AddMainTerminalRemote`).
    pub(crate) fn add_remote_terminal(
        &mut self,
        connection: u32,
        program: ProgramId,
        computer: ComputerId,
    ) {
        self.push_to_root(connection, program, computer, |root| {
            &mut root.remote_terminals
        });
    }

    /// Appends `computer` to the list `list` picks of the root of the tree of `connection`'s `program`.
    fn push_to_root(
        &mut self,
        connection: u32,
        program: ProgramId,
        computer: ComputerId,
        list: impl FnOnce(&mut Program) -> &mut Vec<ComputerId>,
    ) {
        let Some(tree) = self.tree(connection, program) else {
            return;
        };

        if let Some(root) = self.programs.get_mut(&tree.root) {
            list(root).push(computer);
        }
    }

    /// Returns the host of the terminal that `connection`'s `program` is, when it is one.
    pub(crate) fn terminal_host(&self, connection: u32, program: ProgramId) -> Option<ComputerId> {
        self.programs
            .get(&program.id)
            .filter(|entry| entry.connection == connection)
            .and_then(|entry| entry.terminal_info)
            .map(|info| info.host)
    }

    /// Ends the last terminal the tree started, when `connection`'s `program` is a terminal that exited, and returns
    /// what it closes (`HelperScript.CloseRemotePrograms`). A terminal on the computer it came from, with the
    /// bounces of its launcher, closes nothing. `player` is the player's computer, where the first terminal's
    /// connection comes from.
    pub(crate) fn exit_terminal(
        &mut self,
        connection: u32,
        program: ProgramId,
        player: ComputerId,
    ) -> Option<TerminalClose> {
        let info = self
            .programs
            .get(&program.id)
            .filter(|entry| entry.connection == connection)
            .and_then(|entry| entry.terminal_info)?;
        let tree = self.tree(connection, program)?;
        let root = self.programs.get_mut(&tree.root)?;
        let remote = root.remote_terminals.pop()?;
        let source = root.remote_terminals.last().copied().unwrap_or(player);

        (remote != source || info.bounces != info.launcher_bounces)
            .then_some(TerminalClose { source, remote })
    }

    /// Returns the process of `connection`'s active `program` when a terminal launched it, the command the terminal
    /// runs (`HelperScript.subPID`); otherwise `None`.
    pub(crate) fn terminal_command_process(
        &self,
        connection: u32,
        program: ProgramId,
    ) -> Option<Process> {
        let entry = self
            .programs
            .get(&program.id)
            .filter(|entry| entry.connection == connection && entry.stage == Stage::Active)?;
        let launcher = self.programs.get(&entry.parent?)?;

        launcher.terminal.then_some(entry.process).flatten()
    }

    /// Returns the session of `connection`'s active `program`, otherwise `None`.
    pub(crate) fn session(&self, connection: u32, program: ProgramId) -> Option<ObjectId> {
        self.programs
            .get(&program.id)
            .filter(|entry| entry.connection == connection && entry.stage == Stage::Active)
            .map(|entry| entry.session)
    }

    /// Makes `session` the shell session of `connection`'s active `program`.
    pub(crate) fn set_session(&mut self, connection: u32, program: ProgramId, session: ObjectId) {
        if let Some(entry) = self
            .programs
            .get_mut(&program.id)
            .filter(|entry| entry.connection == connection && entry.stage == Stage::Active)
        {
            entry.session = session;
        }
    }

    /// Returns the connection and root ID for the tree of `connection`'s `program`.
    pub(crate) fn tree(&self, connection: u32, program: ProgramId) -> Option<Tree> {
        self.programs
            .get(&program.id)
            .filter(|entry| entry.connection == connection)
            .map(|entry| Tree {
                connection,
                root: entry.root,
            })
    }

    /// Removes and returns the process of `connection`'s active launched `program`, closed by
    /// `start_terminal` (`ShellIntrinsics.start_terminal`). The program continues without a process.
    pub(crate) fn take_launched_process(
        &mut self,
        connection: u32,
        program: ProgramId,
    ) -> Option<Process> {
        self.programs
            .get_mut(&program.id)
            .filter(|entry| {
                entry.connection == connection
                    && entry.stage == Stage::Active
                    && entry.parent.is_some()
            })
            .and_then(|entry| entry.process.take())
    }

    /// Adds `computer` to the terminal list for `connection`'s `program` until its tree ends
    /// (`HelperScript.AddPendingShell`).
    pub(crate) fn add_terminal_computer(
        &mut self,
        connection: u32,
        program: ProgramId,
        computer: ComputerId,
    ) {
        self.push_to_root(connection, program, computer, |root| {
            &mut root.terminal_computers
        });
    }

    /// Returns terminal computers for the tree of `connection`'s `program`.
    pub(crate) fn terminal_computers(
        &self,
        connection: u32,
        program: ProgramId,
    ) -> Vec<ComputerId> {
        self.tree(connection, program)
            .and_then(|tree| self.programs.get(&tree.root))
            .map(|root| root.terminal_computers.clone())
            .unwrap_or_default()
    }

    /// Returns the root process for `connection`'s `program` (`GreyInterpreter.PID`), or `None` when absent.
    pub(crate) fn root_process(&self, connection: u32, program: ProgramId) -> Option<Process> {
        self.tree(connection, program)
            .and_then(|tree| self.programs.get(&tree.root))
            .and_then(|root| root.process)
    }

    /// Returns launch depth of `program`, with 0 for runtime-started programs.
    pub(crate) fn depth(&self, program: ProgramId) -> u32 {
        self.programs
            .get(&program.id)
            .map_or(0, |entry| entry.depth)
    }

    /// Checks for programs on `connection` that are not tombstones.
    pub(crate) fn has_programs(&self, connection: u32) -> bool {
        self.programs
            .values()
            .any(|program| program.connection == connection && program.stage != Stage::Tombstone)
    }

    /// Handles runtime termination of `connection`'s `program`. Active programs end with active descendants; terminating
    /// programs and their terminating descendants become tombstones without separate child acknowledgments.
    /// Returns `None` for tombstones or programs owned by another connection.
    pub(crate) fn end(&mut self, connection: u32, program: ProgramId) -> Option<ClientEnd> {
        let entry = self.programs.get_mut(&program.id)?;

        if entry.connection != connection || entry.stage == Stage::Tombstone {
            return None;
        }

        let running = entry.stage == Stage::Active;

        entry.stage = Stage::Tombstone;

        if !running {
            self.acknowledge_children(program.id);

            return Some(ClientEnd {
                process: None,
                children: Vec::new(),
            });
        }

        let process = entry.process;
        let children = self.end_children(&[program.id]);

        Some(ClientEnd { process, children })
    }

    /// Converts all terminating descendants of `parent` to [`Stage::Tombstone`].
    fn acknowledge_children(&mut self, parent: u32) {
        let mut parents = vec![parent];

        while let Some(parent) = parents.pop() {
            for (&id, program) in &mut self.programs {
                if program.parent == Some(parent) && program.stage == Stage::Termination {
                    program.stage = Stage::Tombstone;
                    parents.push(id);
                }
            }
        }
    }

    /// Lists active program processes hosted on any of `computers`.
    pub(crate) fn processes_on(&self, computers: &[ComputerId]) -> Vec<(ProgramId, Process)> {
        self.programs
            .iter()
            .filter(|(_, program)| program.stage == Stage::Active)
            .filter_map(|(&id, program)| {
                program
                    .process
                    .filter(|process| computers.contains(&process.0))
                    .map(|process| (ProgramId { id }, process))
            })
            .collect()
    }

    /// Terminates `gone` programs and their active descendants, returning all termination records.
    pub(crate) fn take_terminations(
        &mut self,
        gone: &[(ProgramId, EndReason)],
    ) -> Vec<Termination> {
        let mut terminations: Vec<Termination> = gone
            .iter()
            .filter_map(|&(program, reason)| self.terminate(program.id, reason))
            .collect();
        let ids: Vec<u32> = terminations
            .iter()
            .map(|termination| termination.program.id)
            .collect();

        terminations.extend(self.end_children(&ids));

        terminations
    }

    /// Terminates all active descendants of `parents` with [`EndReason::ParentEnd`].
    fn end_children(&mut self, parents: &[u32]) -> Vec<Termination> {
        let mut parents = parents.to_vec();
        let mut terminations = Vec::new();
        let mut index = 0;

        while let Some(&parent) = parents.get(index) {
            let children: Vec<u32> = self
                .programs
                .iter()
                .filter(|(_, program)| {
                    program.parent == Some(parent) && program.stage == Stage::Active
                })
                .map(|(&child, _)| child)
                .collect();

            for child in children {
                if let Some(termination) = self.terminate(child, EndReason::ParentEnd) {
                    terminations.push(termination);
                    parents.push(child);
                }
            }

            index += 1;
        }

        terminations
    }

    /// Moves active program `id` to [`Stage::Termination`] with `reason`.
    fn terminate(&mut self, id: u32, reason: EndReason) -> Option<Termination> {
        let program = self
            .programs
            .get_mut(&id)
            .filter(|program| program.stage == Stage::Active)?;
        let channel = self.connections.get(&program.connection)?;

        program.stage = Stage::Termination;

        Some(Termination {
            program: ProgramId { id },
            reason,
            process: program.process,
            connection: program.connection,
            channel: Rc::clone(channel),
        })
    }
}

#[cfg(test)]
#[path = "tests/programs.rs"]
mod tests;
