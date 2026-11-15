//! Handles debugger commands and evaluates expressions in stopped mock-world programs.

use greybel_interpreter::{InterpreterError, Location, Pin, Vm};

use super::entry::Entry;
use super::inspection::MockInspection;
use super::{EVALUATION_GLOBAL, MockGame, MockProgram, StartedGame as _};
use crate::breakpoints::BreakpointAction;
use crate::program::{EvaluatedText, Evaluator};
use crate::protocol::{EvaluationFailure, Exit, SourceLocation, StopNotice, StopReason, Variable};
use crate::runtime::{Command, Host};

impl<H: Host> MockProgram<'_, H> {
    /// Serves debugger commands at stopped `location`. Returns `None` to resume and `Some(Exit)` on termination;
    /// passing breakpoints resume without a stop report.
    pub(super) async fn serve_stop(
        &mut self,
        game: &mut MockGame<H>,
        location: Option<Location>,
    ) -> Option<Exit> {
        let reason = self.stop_reason(game, location.as_ref()).await?;
        let state = self.state;

        state.host.report_stop(&StopNotice {
            reason,
            location: location.as_ref().map(SourceLocation::from),
        });

        let mut inspection = MockInspection::default();
        let outcome = loop {
            let command = self.commands.recv().await;
            let vm = game.vm();

            match command {
                None | Some(Command::Terminate) => break Some(Exit::Termination),
                Some(Command::Continue) => {
                    vm.set_breakpoints(false);
                    state.stop_reason.set(None);
                    state.control.resume();
                    break None;
                }
                Some(Command::Step(mode)) => {
                    vm.set_breakpoints(false);
                    state.stop_reason.set(Some(StopReason::Step));
                    state.control.step(mode.into(), vm.depth());
                    break None;
                }
                Some(Command::StackTrace(reply)) => {
                    reply(Ok(MockInspection::stack_trace(vm)));
                }
                Some(Command::Scopes(frame, reply)) => {
                    reply(inspection.scopes(vm, frame));
                }
                Some(Command::Variables(reference, reply)) => {
                    reply(inspection.variables(vm, reference));
                }
                Some(Command::Evaluate(code, reply)) => {
                    let evaluation = self.evaluate(game, &code).await;

                    reply(
                        evaluation
                            .map(|pin| pin.map(|pin| inspection.evaluation(game.vm(), pin, &code))),
                    );
                }
                Some(Command::SetVariable(reference, name, value, reply)) => {
                    let variable = self
                        .set_variable(game, &mut inspection, reference, &name, &value)
                        .await;

                    reply(variable);
                }
                Some(Command::Pause | Command::BreakpointChange) => {}
                Some(command @ Command::Launch(..)) => command.refuse(),
            }
        };

        if let Some(vm) = game.interpreter_mut().vm_mut() {
            inspection.release(vm);
        }

        outcome
    }

    /// Classifies debugger, pause, step, and breakpoint stops. Evaluates breakpoint rules and returns `None` when
    /// execution should continue.
    async fn stop_reason(
        &mut self,
        game: &mut MockGame<H>,
        location: Option<&Location>,
    ) -> Option<StopReason> {
        let state = self.state;

        if game.interpreter().vm().is_some_and(Vm::breakpoints) {
            state.stop_reason.set(None);

            return Some(StopReason::Debugger);
        }

        if state.control.stop_reason() != greybel_interpreter::StopReason::Breakpoint {
            return Some(state.stop_reason.take().unwrap_or(StopReason::Pause));
        }

        if let Some(location) = location {
            let mut evaluator = MockEvaluator {
                program: self,
                game: &mut *game,
            };
            let action = state
                .breakpoint_action(&location.uri, location.line, &mut evaluator)
                .await;

            if action == BreakpointAction::Pass {
                return None;
            }
        }

        state.stop_reason.set(None);
        state.control.resume();

        Some(StopReason::Breakpoint)
    }

    /// Assigns the referenced target and returns its updated variable.
    async fn set_variable(
        &self,
        game: &mut MockGame<H>,
        inspection: &mut MockInspection,
        reference: u32,
        name: &str,
        value: &str,
    ) -> Result<Variable, EvaluationFailure> {
        let target = inspection.assignable(game.vm(), reference, name)?;

        self.evaluate(game, &format!("{target} = ({value})"))
            .await?;

        inspection.find_variable(game.vm(), reference, name)
    }

    /// Tries `code` as an expression wrapped in the capture native and returns its pinned value. Otherwise executes
    /// statements and returns `None`.
    async fn evaluate(
        &self,
        game: &mut MockGame<H>,
        code: &str,
    ) -> Result<Option<Pin>, EvaluationFailure> {
        let expression = self
            .state
            .run_with_yields(
                game.interpreter_mut()
                    .inject(&format!("{EVALUATION_GLOBAL}({code})")),
            )
            .await;

        game.send_output();

        let interpreter = game.interpreter_mut();
        let pin = self.evaluation_value.borrow_mut().take();

        match expression {
            Ok(()) => return Ok(pin),
            Err(InterpreterError::Runtime(error)) => {
                if let (Some(pin), Some(vm)) = (pin, interpreter.vm_mut()) {
                    vm.unpin(&pin);
                }

                return Err(EvaluationFailure::from_message(error.to_string()));
            }
            Err(_) => {}
        }

        let statements = self.state.run_with_yields(interpreter.inject(code)).await;

        game.send_output();

        statements
            .map(|()| None)
            .map_err(|error| EvaluationFailure::from_message(error.to_string()))
    }
}

/// Breakpoint evaluator for a stopped mock program.
struct MockEvaluator<'p, 'r, H: Host> {
    program: &'p MockProgram<'r, H>,
    game: &'p mut MockGame<H>,
}

impl<H: Host> Evaluator for MockEvaluator<'_, '_, H> {
    async fn evaluate_text(
        &mut self,
        code: &str,
    ) -> Result<Option<EvaluatedText>, EvaluationFailure> {
        let Some(pin) = self.program.evaluate(self.game, code).await? else {
            return Ok(None);
        };
        let vm = self.game.vm();
        let value = vm.unpin(&pin);

        Ok(Some(EvaluatedText {
            text: Entry::display(vm.heap(), &value),
            type_name: value.type_name().into(),
        }))
    }
}
