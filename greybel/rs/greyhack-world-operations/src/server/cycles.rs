//! Cycle-driver implementation available to wait routes.

use std::any::Any;

use greyhack_world::World;
use greyhack_world_core::{ComputerId, WorldError};
use greyhack_world_router::{
    Call, CycleDriver, CycleFuture, ErasedCycleCall, ErasedWorldCall, Process, ProgramKind,
    RouteContext, RouteResult, TreeId,
};

use super::{ServerState, WorldState};
use crate::OperationError;
use crate::protocol::{ObjectId, ProgramId, ScriptFailure};

impl CycleDriver<World, WorldError> for ServerState {
    fn change<'a>(
        &'a self,
        connection: u32,
        program: ProgramId,
        budgeted: bool,
        apply: Box<dyn ErasedCycleCall<World, WorldError> + 'a>,
    ) -> CycleFuture<'a, RouteResult<Box<dyn Any>, WorldError>> {
        Box::pin(async move {
            let run = async |world_state: &mut WorldState| {
                let Some(key) = self.trees.of_program(program) else {
                    return Err(OperationError::Script(ScriptFailure::ProgramEnded));
                };

                if !world_state
                    .tree(key)
                    .is_some_and(|tree| tree.active(program).is_some())
                {
                    return Err(OperationError::Script(ScriptFailure::ProgramEnded));
                }

                let context =
                    RouteContext::new(world_state.world.clone(), world_state.trees.clone())
                        .with_prints(self.prints.clone());

                apply.run(&context, Call { tree: key, program }).await
            };

            if budgeted {
                self.cycle_for(connection, program, run).await
            } else {
                self.cycle(run, true).await
            }
        })
    }

    fn world<'a>(
        &'a self,
        apply: Box<dyn ErasedWorldCall<World> + 'a>,
    ) -> CycleFuture<'a, Box<dyn Any>> {
        Box::pin(async move {
            self.cycle(
                async |state: &mut WorldState| apply.run(&state.world).await,
                true,
            )
            .await
        })
    }

    fn push(
        &self,
        tree: TreeId,
        process: Option<Process<ComputerId>>,
        session: ObjectId,
        parent: ProgramId,
        kind: ProgramKind<ComputerId>,
    ) -> RouteResult<ProgramId, WorldError> {
        let connection = self
            .trees
            .get(tree)
            .map(|tree| tree.connection())
            .ok_or(OperationError::Script(ScriptFailure::ProgramEnded))?;
        let (program, _) = self
            .index
            .borrow_mut()
            .add_program(connection)
            .ok_or(OperationError::Script(ScriptFailure::ProgramEnded))?;

        self.trees
            .push(tree, program, process, session, parent, kind);

        Ok(program)
    }

    fn defer<'a>(
        &'a self,
        connection: u32,
        program: ProgramId,
        work: u64,
    ) -> CycleFuture<'a, Option<Box<dyn Any>>> {
        Box::pin(async move {
            self.defer_envelope(connection, program, work)
                .await
                .ok()?
                .into_result()
                .ok()
        })
    }

    fn routine_budget(&self) -> usize {
        self.routine_budget()
    }

    fn save<'a>(&'a self) -> CycleFuture<'a, bool> {
        Box::pin(self.save())
    }
}
