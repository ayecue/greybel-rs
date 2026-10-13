//! Program and connection termination.

use std::rc::Rc;

use greyhack_world::{ComputerId, World};

use super::programs::{Process, Termination};
use super::{ServerState, WorldState};
use crate::protocol::{ClientEnd, EndReason, ObjectId, ProgramEnd, ProgramId, ScriptFailure};
use crate::{OperationError, Result, connection};

impl ServerState {
    /// Terminates active programs whose processes disappeared on changed computers, including descendants
    /// and deferred jobs, and queues [`ProgramEnd`]s. Uses [`EndReason::ComputerRemoval`] for removed
    /// computers, [`EndReason::Reboot`] for computers in `reboots`, and [`EndReason::Closed`] otherwise.
    pub(super) async fn terminate_ended_programs(&self, world: &mut World, reboots: &[ComputerId]) {
        let processes = self
            .programs
            .borrow()
            .processes_on(&world.take_changed_computers());
        let mut gone = Vec::new();

        for (program, (computer, pid)) in processes {
            let process = match world.process(computer, pid).await {
                Ok(process) => process,
                Err(error) => {
                    self.record_failure(error);

                    return;
                }
            };

            match world.computer(computer).await {
                Ok(Some(_)) if process.is_some() => {}
                Ok(Some(_)) if reboots.contains(&computer) => {
                    gone.push((program, EndReason::Reboot))
                }
                Ok(Some(_)) => gone.push((program, EndReason::Closed)),
                Ok(None) => gone.push((program, EndReason::ComputerRemoval)),
                Err(error) => {
                    self.record_failure(error);

                    return;
                }
            }
        }

        if gone.is_empty() {
            return;
        }

        let terminations = self.programs.borrow_mut().take_terminations(&gone);

        self.end_terminations(world, terminations).await;
    }

    /// Cancels jobs and processes for `terminations`, then queues their [`ProgramEnd`] notifications.
    pub(super) async fn end_terminations(&self, world: &mut World, terminations: Vec<Termination>) {
        for termination in &terminations {
            if let Err(error) = self
                .cancel_jobs(world, |job| {
                    job.connection == termination.connection && job.program == termination.program
                })
                .await
            {
                self.record_failure(error);
            }

            if let Err(error) = world
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
        if reason == ClientEnd::Exited
            && let Err(error) = self
                .close_terminal(&mut state.world, connection, program)
                .await
        {
            self.record_failure(error);
        }

        let Some(end) = self.programs.borrow_mut().end(connection, program) else {
            return;
        };

        if let Err(error) = self
            .cancel_jobs(&mut state.world, |job| {
                job.connection == connection && job.program == program
            })
            .await
        {
            self.record_failure(error);
        }

        if let Err(error) = state
            .world
            .end_program_effects(end.process, u64::from(program.id))
            .await
        {
            self.record_failure(error);
        }

        self.end_terminations(&mut state.world, end.children).await;
    }

    /// Logs the exit of the terminal that `connection`'s `program` is on the computer it ran on, as
    /// `MainClass` does when the terminal's bash ends with `exit` (`HelperScript.CloseRemotePrograms`). Nothing
    /// happens when the terminal's computer or the tree's root process is gone.
    pub(super) async fn close_terminal(
        &self,
        world: &mut World,
        connection: u32,
        program: ProgramId,
    ) -> greyhack_world::Result<()> {
        let (host, root) = {
            let programs = self.programs.borrow();

            (
                programs.terminal_host(connection, program),
                programs.root_process(connection, program),
            )
        };
        let (Some(host), Some((root_computer, root_pid))) = (host, root) else {
            return Ok(());
        };

        if !world
            .is_terminal_running(host, (root_computer, root_pid))
            .await?
        {
            return Ok(());
        }

        let player = world.player();
        let close = self
            .programs
            .borrow_mut()
            .exit_terminal(connection, program, player);
        let Some(close) = close else {
            return Ok(());
        };

        world.log_terminal_exit(close.source, close.remote).await
    }

    /// Sends all queued program termination notices.
    pub(super) fn send_program_ends(&self) {
        let ends = std::mem::take(&mut *self.program_ends.borrow_mut());

        for (end, channel) in ends {
            let _ = channel.send_program_end(&end);
        }
    }

    /// Registers a program on `connection` with `process`, shell `session`, and optional launcher `parent`.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptFailure::ProgramEnded`] after program IDs are exhausted.
    pub(crate) fn add_program(
        &self,
        connection: u32,
        process: Option<Process>,
        session: ObjectId,
        parent: Option<ProgramId>,
    ) -> Result<ProgramId> {
        self.programs
            .borrow_mut()
            .add_program(connection, process, session, parent)
            .ok_or(OperationError::Script(ScriptFailure::ProgramEnded))
    }

    /// Registers a `start_terminal` program on `connection` with shell `session`, launched by `parent`. It has no
    /// process.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptFailure::ProgramEnded`] after program IDs are exhausted.
    pub(crate) fn add_terminal_program(
        &self,
        connection: u32,
        session: ObjectId,
        parent: ProgramId,
        info: super::TerminalInfo,
    ) -> Result<ProgramId> {
        let program = self.add_program(connection, None, session, Some(parent))?;

        self.programs.borrow_mut().set_terminal(program, info);

        Ok(program)
    }

    /// Checks whether `connection` has any non-tombstone programs.
    pub(crate) fn has_programs(&self, connection: u32) -> bool {
        self.programs.borrow().has_programs(connection)
    }

    /// Schedules cleanup after active operations: remove connection objects and end its program processes.
    pub(super) fn end_connection(self: &Rc<Self>, connection: u32, spawn: &connection::Spawn) {
        let (channel, processes) = self.programs.borrow_mut().remove_connection(connection);

        self.budgets.remove_connection(connection);

        drop(channel);

        let state = Rc::clone(self);

        spawn(Box::pin(async move {
            state
                .cycle(async |world_state| {
                    world_state.objects.remove(&connection);
                    if let Err(error) = state
                        .cancel_jobs(&mut world_state.world, |job| job.connection == connection)
                        .await
                    {
                        state.record_failure(error);
                    }

                    if let Err(error) = world_state.world.end_orphaned_processes(&processes).await {
                        state.record_failure(error);
                    }
                })
                .await;
        }));
    }
}
