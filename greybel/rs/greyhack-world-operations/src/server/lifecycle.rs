//! Program and connection termination.

use std::collections::HashMap;
use std::rc::Rc;

use greyhack_world::ComputerId;
use greyhack_world_router::{
    Process, ProgramEnd as TreeProgramEnd, ProgramTree, TerminalClose, TreeId,
};

use super::{ConnectionChannel, ServerState, WorldState};
use crate::protocol::{ClientEnd, EndReason, ObjectId, ProgramEnd, ProgramId, ScriptFailure};
use crate::{OperationError, Result, connection};

/// Server-side program termination and its notification channel.
pub(crate) struct Termination {
    pub(crate) program: ProgramId,
    pub(crate) reason: EndReason,
    pub(crate) process: Option<Process<ComputerId>>,
    pub(crate) connection: u32,
    pub(crate) channel: ConnectionChannel,
}

impl ServerState {
    /// Terminates active programs whose processes disappeared on changed computers, including descendants
    /// and deferred jobs, and queues [`ProgramEnd`]s. Uses [`EndReason::ComputerRemoval`] for removed
    /// computers, [`EndReason::Reboot`] for computers in `reboots`, and [`EndReason::Closed`] otherwise.
    pub(super) async fn terminate_ended_programs(
        &self,
        state: &mut WorldState,
        reboots: &[ComputerId],
    ) {
        let computers = state.world.take_changed_computers();
        let processes = state.trees.processes_on(&computers);
        let mut programs_by_tree: HashMap<TreeId, Vec<(ProgramId, EndReason)>> = HashMap::new();

        for (key, program, (computer, pid)) in processes {
            let process = match state
                .world
                .modules
                .core
                .computers
                .processes
                .process(computer, pid)
                .await
            {
                Ok(process) => process,
                Err(error) => {
                    self.record_failure(error);

                    return;
                }
            };

            match state
                .world
                .modules
                .core
                .networks
                .topology
                .computer(computer)
                .await
            {
                Ok(Some(_)) if process.is_some() => {}
                Ok(Some(_)) if reboots.contains(&computer) => {
                    programs_by_tree
                        .entry(key)
                        .or_default()
                        .push((program, EndReason::Reboot));
                }
                Ok(Some(_)) => {
                    programs_by_tree
                        .entry(key)
                        .or_default()
                        .push((program, EndReason::Closed));
                }
                Ok(None) => {
                    programs_by_tree
                        .entry(key)
                        .or_default()
                        .push((program, EndReason::ComputerRemoval));
                }
                Err(error) => {
                    self.record_failure(error);

                    return;
                }
            }
        }

        if programs_by_tree.is_empty() {
            return;
        }

        let mut terminations = Vec::new();

        for (key, programs_by_tree) in programs_by_tree {
            let ends = state
                .tree_mut(key)
                .expect("the scanned tree exists")
                .take_terminations(&programs_by_tree);

            terminations.extend(self.with_channels(key, ends));
        }

        self.end_terminations(state, terminations).await;
    }

    /// Adds notification channels to tree termination records.
    fn with_channels(
        &self,
        key: TreeId,
        ends: Vec<TreeProgramEnd<ComputerId>>,
    ) -> Vec<Termination> {
        ends.into_iter()
            .filter_map(|end| {
                Some(Termination {
                    program: end.program,
                    reason: end.reason,
                    process: end.process,
                    connection: self.trees.get(key)?.connection(),
                    channel: self
                        .index
                        .borrow()
                        .channel(key, self.trees.get(key)?.connection())?,
                })
            })
            .collect()
    }

    /// Cancels jobs and processes for `terminations`, then queues their [`ProgramEnd`] notifications.
    pub(super) async fn end_terminations(
        &self,
        state: &mut WorldState,
        terminations: Vec<Termination>,
    ) {
        for termination in &terminations {
            if let Err(error) = self
                .cancel_jobs(state, |job| {
                    job.connection == termination.connection && job.program == termination.program
                })
                .await
            {
                self.record_failure(error);
            }

            if let Err(error) = state
                .world
                .modules
                .shell
                .shells
                .end_program_effects(termination.process, u64::from(termination.program.id))
                .await
            {
                self.record_failure(error);
            }
        }

        self.program_ends
            .borrow_mut()
            .extend(terminations.into_iter().map(|termination| {
                (
                    ProgramEnd {
                        program: termination.program,
                        reason: termination.reason,
                    },
                    termination.channel,
                )
            }));
    }

    /// Handles runtime-requested termination of `connection`'s `program`, cancelling its process,
    /// deferred jobs, and active descendants in the current cycle.
    pub(crate) async fn end_program(
        &self,
        state: &mut WorldState,
        connection: u32,
        program: ProgramId,
        reason: ClientEnd,
    ) {
        let key = self.trees.of_program(program);

        if reason == ClientEnd::Exited
            && let Some(key) = key
            && let Err(error) = self.close_terminal(state, key, program).await
        {
            self.record_failure(error);
        }

        let Some(key) = key else {
            return;
        };
        let Some(end) = state
            .tree_mut(key)
            .expect("the indexed tree exists")
            .pop(program)
        else {
            return;
        };

        if let Err(error) = self
            .cancel_jobs(state, |job| {
                job.connection == connection && job.program == program
            })
            .await
        {
            self.record_failure(error);
        }

        if let Err(error) = state
            .world
            .modules
            .shell
            .shells
            .end_program_effects(end.process, u64::from(program.id))
            .await
        {
            self.record_failure(error);
        }

        let children = self.with_channels(key, end.children);

        self.end_terminations(state, children).await;
    }

    /// Logs the exit of the terminal `program` on the computer it ran on, as `MainClass` does when the
    /// terminal's bash ends with `exit` (`HelperScript.CloseRemotePrograms`). Nothing happens when the
    /// terminal's computer or the tree's root process is programs_by_tree.
    pub(super) async fn close_terminal(
        &self,
        state: &mut WorldState,
        key: TreeId,
        program: ProgramId,
    ) -> greyhack_world::Result<()> {
        let Some((host, root)) = state
            .tree(key)
            .map(|tree| (tree.terminal_host(program), tree.root_process()))
        else {
            return Ok(());
        };
        let (Some(host), Some((root_computer, root_pid))) = (host, root) else {
            return Ok(());
        };

        if !state
            .world
            .modules
            .shell
            .shells
            .is_terminal_running(host, (root_computer, root_pid))
            .await?
        {
            return Ok(());
        }

        let player = state.world.handle.player();
        let close = state
            .tree_mut(key)
            .expect("the indexed tree exists")
            .exit_terminal(program, player);
        let Some(TerminalClose { source, remote }) = close else {
            return Ok(());
        };

        state
            .world
            .modules
            .shell
            .shells
            .log_terminal_exit(source, remote)
            .await
    }

    /// Closes the terminal that `connection`'s `program` runs in when an operation lost its connection:
    /// `result` is [`ScriptFailure::ConnectionInterrupted`] (`GreyInterpreter.interruptConnection`, from
    /// `HelperScript.CheckBreakConnecion`), or `rebooted` says the terminal's own computer rebooted
    /// (`GreyInterpreter.exitBashReboot`). A reboot also ends the terminal's launcher and their descendants.
    pub(crate) async fn close_lost_terminal<T>(
        &self,
        state: &mut WorldState,
        _connection: u32,
        program: ProgramId,
        result: &Result<T>,
        rebooted: bool,
    ) {
        let interrupted = matches!(
            result,
            Err(OperationError::Script(ScriptFailure::ConnectionInterrupted))
        );

        if !interrupted && !rebooted {
            return;
        }

        let Some(key) = self.trees.of_program(program) else {
            return;
        };
        let Some(terminal) = state.tree(key).and_then(|tree| tree.terminal_of(program)) else {
            return;
        };

        if let Err(error) = self.close_terminal(state, key, terminal).await {
            self.record_failure(error);
        }

        if rebooted {
            let ends = {
                let mut tree = state.tree_mut(key).expect("the indexed tree exists");
                let mut reboot_ends = vec![(terminal, EndReason::Reboot)];

                reboot_ends.extend(
                    tree.launcher_of(terminal)
                        .map(|launcher| (launcher, EndReason::Reboot)),
                );

                tree.take_terminations(&reboot_ends)
            };
            let terminations = self.with_channels(key, ends);

            self.end_terminations(state, terminations).await;
        }
    }

    /// Sends all queued program termination notices.
    pub(super) fn send_program_ends(&self) {
        let ends = std::mem::take(&mut *self.program_ends.borrow_mut());

        for (end, channel) in ends {
            let _ = channel.send_program_end(&end);
        }
    }

    /// Registers a runtime-started root program on `connection` with `process` and shell `session`.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptFailure::ProgramEnded`] after program IDs are exhausted.
    pub(crate) fn add_root(
        &self,
        state: &mut WorldState,
        connection: u32,
        process: Option<Process<ComputerId>>,
        session: ObjectId,
        safe_terminal: bool,
    ) -> Result<ProgramId> {
        let (program, _key) = self
            .index
            .borrow_mut()
            .add_program(connection)
            .ok_or(OperationError::Script(ScriptFailure::ProgramEnded))?;

        state.trees.insert(ProgramTree::new(
            connection,
            program,
            process,
            session,
            safe_terminal,
        ));

        Ok(program)
    }

    /// Checks whether `connection` has any non-tombstone programs.
    pub(crate) async fn has_programs(&self, connection: u32) -> bool {
        self.trees.has_programs(connection)
    }

    /// Schedules cleanup after active operations: remove connection trees and end their program processes.
    pub(super) fn end_connection(self: &Rc<Self>, connection: u32, spawn: &connection::Spawn) {
        let channel = self.index.borrow_mut().remove_connection(connection);

        self.budgets.remove(&self.trees.on_connection(connection));

        drop(channel);

        let state = Rc::clone(self);

        spawn(Box::pin(async move {
            state
                .cycle(
                    async |world_state| {
                        let processes = world_state.trees.remove_connection(connection);

                        if let Err(error) = state
                            .cancel_jobs(world_state, |job| job.connection == connection)
                            .await
                        {
                            state.record_failure(error);
                        }

                        if let Err(error) = world_state
                            .world
                            .modules
                            .shell
                            .shells
                            .end_orphaned_processes(&processes)
                            .await
                        {
                            state.record_failure(error);
                        }
                    },
                    true,
                )
                .await;
        }));
    }
}
