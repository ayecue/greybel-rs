use std::future::Future;
use std::rc::Rc;

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;

use super::command::{Command, Reply};
use super::host::Host;
use super::state::RuntimeState;
use crate::protocol::{
    Breakpoint, Evaluation, EvaluationFailure, Exit, InspectionFailure, Launch, LaunchFailure,
    Scope, SourceBreakpoint, StackFrame, StepMode, Variable,
};
use crate::uri::EditorUri as _;

/// Launch and debugger control handle. Commands run in submission order, so pre-launch pauses take effect when
/// the program starts.
pub struct Controller<H: Host> {
    state: Rc<RuntimeState<H>>,
    commands: UnboundedSender<Command>,
}

impl<H: Host> Clone for Controller<H> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
            commands: self.commands.clone(),
        }
    }
}

impl<H: Host> Controller<H> {
    pub(super) fn new(state: Rc<RuntimeState<H>>, commands: UnboundedSender<Command>) -> Self {
        Self { state, commands }
    }

    /// Launches the program and resolves at exit, retaining existing breakpoints.
    pub fn launch(
        &self,
        launch: Launch<'static>,
    ) -> impl Future<Output = Result<Exit, LaunchFailure>> + 'static {
        let result = self.request(|reply| Command::Launch(launch, reply));

        async move { result.await.unwrap_or(Ok(Exit::Termination)) }
    }

    /// Replaces breakpoints at canonical `uri`, updates debugger state, and returns verification results in input order.
    pub fn set_breakpoints(&self, uri: &str, breakpoints: &[SourceBreakpoint]) -> Vec<Breakpoint> {
        let uri = uri.canonical();
        let mut table = self.state.breakpoints.borrow_mut();
        let set = table.set(uri.clone(), breakpoints);

        self.state.control.set_breakpoints(&uri, table.lines(&uri));
        drop(table);
        self.queue(Command::BreakpointChange);

        set
    }

    /// Resumes the stopped program until its next stop or exit.
    pub fn resume(&self) {
        self.queue(Command::Continue);
    }

    /// Requests a debugger step with `mode`.
    pub fn step(&self, mode: StepMode) {
        self.queue(Command::Step(mode));
    }

    /// Pauses the running program at its next statement.
    pub fn pause(&self) {
        self.queue(Command::Pause);
    }

    /// Terminates the active program and resolves its launch with [`Exit::Termination`].
    pub fn terminate(&self) {
        self.queue(Command::Terminate);
    }

    /// Returns the stopped call stack, innermost frame first.
    pub fn stack_trace(
        &self,
    ) -> impl Future<Output = Result<Vec<StackFrame>, InspectionFailure>> + 'static {
        self.inspect(Command::StackTrace)
    }

    /// Returns scopes for stopped frame `frame` ([`StackFrame::id`]).
    pub fn scopes(
        &self,
        frame: u32,
    ) -> impl Future<Output = Result<Vec<Scope>, InspectionFailure>> + 'static {
        self.inspect(move |reply| Command::Scopes(frame, reply))
    }

    /// Returns variables in the referenced scope, list, or map.
    pub fn variables(
        &self,
        reference: u32,
    ) -> impl Future<Output = Result<Vec<Variable>, InspectionFailure>> + 'static {
        self.inspect(move |reply| Command::Variables(reference, reply))
    }

    /// Evaluates `code` in the stopped innermost frame; expressions return values and statements return `None`.
    pub fn evaluate(
        &self,
        code: String,
    ) -> impl Future<Output = Result<Option<Evaluation>, EvaluationFailure>> + 'static {
        self.evaluation(move |reply| Command::Evaluate(code, reply))
    }

    /// Evaluates `value`, assigns the referenced variable or collection item, and returns the updated variable.
    pub fn set_variable(
        &self,
        reference: u32,
        name: String,
        value: String,
    ) -> impl Future<Output = Result<Variable, EvaluationFailure>> + 'static {
        self.evaluation(move |reply| Command::SetVariable(reference, name, value, reply))
    }

    /// Sends `command` to the runtime loop or rejects it after loop termination.
    pub(crate) fn queue(&self, command: Command) {
        if let Err(error) = self.commands.send(command) {
            error.0.refuse();
        }
    }

    /// Queues a request with a one-shot reply; dropped commands return `None`.
    fn request<T: 'static>(
        &self,
        command: impl FnOnce(Reply<T>) -> Command,
    ) -> impl Future<Output = Option<T>> + 'static {
        let (sender, receiver) = oneshot::channel();

        self.queue(command(Box::new(move |result| {
            let _ = sender.send(result);
        })));

        async move { receiver.await.ok() }
    }

    fn inspect<T: 'static>(
        &self,
        command: impl FnOnce(Reply<Result<T, InspectionFailure>>) -> Command,
    ) -> impl Future<Output = Result<T, InspectionFailure>> + 'static {
        let result = self.request(command);

        async move {
            result
                .await
                .unwrap_or(Err(InspectionFailure::ActiveProgram))
        }
    }

    fn evaluation<T: 'static>(
        &self,
        command: impl FnOnce(Reply<Result<T, EvaluationFailure>>) -> Command,
    ) -> impl Future<Output = Result<T, EvaluationFailure>> + 'static {
        let result = self.request(command);

        async move {
            result
                .await
                .unwrap_or(Err(EvaluationFailure::ActiveProgram))
        }
    }
}
