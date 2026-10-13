use tokio::sync::mpsc::UnboundedReceiver;

use crate::protocol::{
    Evaluation, EvaluationFailure, Exit, InspectionFailure, Launch, LaunchFailure, Scope,
    StackFrame, StepMode, Variable,
};

/// One-shot callback for a command result.
pub(crate) type Reply<T> = Box<dyn FnOnce(T)>;

/// Controller command delivered to the runtime loop in order.
pub(crate) enum Command {
    Launch(Launch<'static>, Reply<Result<Exit, LaunchFailure>>),
    Continue,
    Step(StepMode),
    Pause,
    Terminate,
    StackTrace(Reply<Result<Vec<StackFrame>, InspectionFailure>>),
    Scopes(u32, Reply<Result<Vec<Scope>, InspectionFailure>>),
    Variables(u32, Reply<Result<Vec<Variable>, InspectionFailure>>),
    Evaluate(String, Reply<Result<Option<Evaluation>, EvaluationFailure>>),
    /// Assigns an evaluated expression to a variable or collection item ([`crate::protocol::SetVariable`]).
    SetVariable(
        u32,
        String,
        String,
        Reply<Result<Variable, EvaluationFailure>>,
    ),
    /// Notifies the active game program of breakpoint changes.
    BreakpointChange,
}

impl Command {
    /// Rejects state-incompatible requests with declared failures. Drops notifications and state-change commands
    /// without replies.
    pub(crate) fn refuse(self) {
        match self {
            Self::Launch(_, reply) => reply(Err(LaunchFailure::Busy)),
            Self::StackTrace(reply) => reply(Err(InspectionFailure::ActiveProgram)),
            Self::Scopes(_, reply) => reply(Err(InspectionFailure::ActiveProgram)),
            Self::Variables(_, reply) => reply(Err(InspectionFailure::ActiveProgram)),
            Self::Evaluate(_, reply) => reply(Err(EvaluationFailure::ActiveProgram)),
            Self::SetVariable(.., reply) => reply(Err(EvaluationFailure::ActiveProgram)),
            Self::Continue
            | Self::Step(_)
            | Self::Pause
            | Self::Terminate
            | Self::BreakpointChange => {}
        }
    }
}

/// FIFO controller-command receiver.
pub(crate) struct CommandQueue(UnboundedReceiver<Command>);

impl CommandQueue {
    pub(super) fn new(commands: UnboundedReceiver<Command>) -> Self {
        Self(commands)
    }

    /// Receives the next command, or `None` once all controllers are dropped.
    pub(crate) async fn recv(&mut self) -> Option<Command> {
        self.0.recv().await
    }

    /// Waits for `Terminate` or queue closure, invoking `pause` for each ordered `Pause` and rejecting other commands.
    /// Thus startup pauses take effect when the program runs.
    pub(crate) async fn wait_for_termination(&mut self, pause: impl Fn()) {
        loop {
            match self.recv().await {
                None | Some(Command::Terminate) => return,
                Some(Command::Pause) => pause(),
                Some(command) => command.refuse(),
            }
        }
    }
}
