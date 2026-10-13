//! Handles debugger commands and evaluates expressions for game-hosted programs.

use greyhack_message_hook_client::protocol::greyhack::ContextBreakpointRpc;
use greyhack_message_hook_client::{Context, DebugValue, Evaluation, Payload};

use super::GameProgram;
use super::inspection::GameInspection;
use crate::breakpoints::BreakpointAction;
use crate::program::{EvaluatedText, Evaluator};
use crate::protocol::{
    Evaluation as ProtocolEvaluation, EvaluationFailure, Exit, InspectionFailure, SourceLocation,
    StopNotice, StopReason, Variable,
};
use crate::runtime::{Command, Host};

impl<H: Host> GameProgram<'_, H> {
    /// Reports stop location and reason to the editor. Passing breakpoints resume in-game without a stop notification.
    pub(super) async fn stop(&mut self, context: &Context, stop: &ContextBreakpointRpc<'_>) {
        let location = SourceLocation::in_game(&stop.filepath, stop.line_num);
        let reason = if stop.breakpoint {
            let mut evaluator = GameEvaluator(context);
            let action = self
                .state
                .breakpoint_action(&location.uri, location.line, &mut evaluator)
                .await;

            if action == BreakpointAction::Pass {
                let _ = context.pass_breakpoint().await;

                return;
            }

            self.state.stop_reason.set(None);

            StopReason::Breakpoint
        } else {
            self.state
                .stop_reason
                .take()
                .unwrap_or(StopReason::Debugger)
        };

        self.inspection = Some(GameInspection::new(
            stop.frames.json().unwrap_or_default(),
            stop.globals.json().unwrap_or_default(),
        ));

        self.state.host.report_stop(&StopNotice {
            reason,
            location: Some(location),
        });
    }

    /// Applies an editor command to the game context. Termination or command-queue closure returns `Some(Exit)`;
    /// otherwise returns `None`.
    pub(super) async fn command(
        &mut self,
        context: &Context,
        command: Option<Command>,
    ) -> Option<Exit> {
        let Some(command) = command else {
            return Some(Exit::Termination);
        };

        match command {
            Command::Terminate => return Some(Exit::Termination),
            Command::Continue if self.inspection.take().is_some() => {
                let _ = context.set_debug_mode(false).await;
            }
            Command::Step(mode) if self.inspection.take().is_some() => {
                self.state.stop_reason.set(Some(StopReason::Step));

                let _ = context.step(mode.into()).await;
            }
            Command::Pause if self.inspection.is_none() && self.launch.debug => {
                self.state.stop_reason.set(Some(StopReason::Pause));

                let _ = context.set_debug_mode(true).await;
            }
            Command::BreakpointChange => {
                let _ = context.set_breakpoints(&self.breakpoints()).await;
            }
            Command::StackTrace(reply) => reply(
                self.inspection
                    .as_ref()
                    .map(GameInspection::stack_trace)
                    .ok_or(InspectionFailure::ActiveProgram),
            ),
            Command::Scopes(frame, reply) => reply(
                self.inspection
                    .as_mut()
                    .ok_or(InspectionFailure::ActiveProgram)
                    .and_then(|inspection| inspection.scopes(frame)),
            ),
            Command::Variables(reference, reply) => {
                reply(self.variables(context, reference).await);
            }
            Command::Evaluate(code, reply) => reply(self.evaluate(context, &code).await),
            Command::SetVariable(reference, name, value, reply) => {
                reply(self.set_variable(context, reference, name, &value).await);
            }
            command @ Command::Launch(..) => command.refuse(),
            Command::Continue | Command::Step(_) | Command::Pause => {}
        }

        None
    }

    /// Returns variables in the referenced scope or value, reevaluating truncated values to load children.
    async fn variables(
        &mut self,
        context: &Context,
        reference: u32,
    ) -> Result<Vec<Variable>, InspectionFailure> {
        let children_expression = self.inspection()?.children_expression(reference);

        if let Some(code) = children_expression
            && let Ok(Evaluation::Value(value)) = context.evaluate(&code).await
        {
            self.inspection()?.take_children(reference, value);
        }

        self.inspection()?.variables(reference)
    }

    /// Evaluates `code` in the stopped game context and converts expression results to protocol evaluations.
    async fn evaluate(
        &mut self,
        context: &Context,
        code: &str,
    ) -> Result<Option<ProtocolEvaluation>, EvaluationFailure> {
        self.inspection()
            .map_err(|_| EvaluationFailure::ActiveProgram)?;

        let value = evaluate_in_game(context, code).await?;
        let inspection = self
            .inspection()
            .map_err(|_| EvaluationFailure::ActiveProgram)?;

        Ok(value.map(|value| inspection.evaluation(value, code)))
    }

    /// Evaluates an assignment and returns the target's updated variable after rereading it.
    async fn set_variable(
        &mut self,
        context: &Context,
        reference: u32,
        name: String,
        value: &str,
    ) -> Result<Variable, EvaluationFailure> {
        let target = self
            .inspection()
            .map_err(|_| EvaluationFailure::ActiveProgram)?
            .assignable(reference, &name)?;

        evaluate_in_game(context, &format!("{target} = ({value})")).await?;

        let value = evaluate_in_game(context, &target)
            .await?
            .unwrap_or_default();
        let inspection = self
            .inspection()
            .map_err(|_| EvaluationFailure::ActiveProgram)?;

        Ok(inspection.variable(name, value, Some(target)))
    }

    /// Returns current stop inspection, or `InspectionFailure::ActiveProgram` if execution is active.
    fn inspection(&mut self) -> Result<&mut GameInspection, InspectionFailure> {
        self.inspection
            .as_mut()
            .ok_or(InspectionFailure::ActiveProgram)
    }
}

/// Evaluates stopped-context `code`: expression values become `Some`, statements `None`, and failures
/// `EvaluationFailure`.
async fn evaluate_in_game(
    context: &Context,
    code: &str,
) -> Result<Option<DebugValue>, EvaluationFailure> {
    match context.evaluate(code).await {
        Ok(Evaluation::Value(value)) => Ok(Some(value)),
        Ok(Evaluation::Statements) => Ok(None),
        Ok(Evaluation::Failure(message)) => Err(EvaluationFailure::from_message(message)),
        Err(error) => Err(EvaluationFailure::from_message(error.to_string())),
    }
}

/// Game-context breakpoint evaluator.
struct GameEvaluator<'c>(&'c Context);

impl Evaluator for GameEvaluator<'_> {
    async fn evaluate_text(
        &mut self,
        code: &str,
    ) -> Result<Option<EvaluatedText>, EvaluationFailure> {
        let value = evaluate_in_game(self.0, code).await?;

        Ok(value.map(|value| EvaluatedText {
            text: value.text,
            type_name: value.type_name,
        }))
    }
}
