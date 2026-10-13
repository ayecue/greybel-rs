use duplex_core::{Driver, NextMessage, NextNotification, Responder, ValidatedEncode};

use super::EditorHost;
use crate::protocol::{
    Breakpoint, Continue, Evaluate, Evaluation, EvaluationFailure, Exit, InspectionFailure, Launch,
    LaunchFailure, Pause, RunnerRouter, Scope, Scopes, SetBreakpoints, SetVariable, StackFrame,
    StackTrace, Step, Terminate, Variable, Variables,
};
use crate::runtime::{Command, Controller, Reply};

/// Routes editor controller runtime messages. Program commands are queued in order; breakpoint updates apply
/// immediately.
pub(super) struct RunnerHandler<D: Driver>(pub(super) Controller<EditorHost<D>>);

/// Creates a callback mapping `Ok` to a response and `Err` to a declared failure.
fn reply<R: ValidatedEncode + 'static, F: ValidatedEncode + 'static>(
    responder: Responder<R, F>,
) -> Reply<Result<R, F>> {
    Box::new(move |result| match result {
        Ok(response) => responder.respond(response),
        Err(failure) => responder.fail(failure),
    })
}

impl<D: Driver> RunnerRouter for RunnerHandler<D> {
    fn on_launch(
        &self,
        message: Launch<'static>,
        responder: Responder<Exit, LaunchFailure>,
        _next: NextMessage<Launch<'static>>,
    ) {
        self.0.queue(Command::Launch(message, reply(responder)));
    }

    fn on_set_breakpoints(
        &self,
        message: SetBreakpoints<'static>,
        responder: Responder<Vec<Breakpoint>>,
        _next: NextMessage<SetBreakpoints<'static>>,
    ) {
        responder.respond(self.0.set_breakpoints(&message.uri, &message.breakpoints));
    }

    fn on_set_variable(
        &self,
        message: SetVariable<'static>,
        responder: Responder<Variable, EvaluationFailure>,
        _next: NextMessage<SetVariable<'static>>,
    ) {
        self.0.queue(Command::SetVariable(
            message.reference,
            message.name.into_owned(),
            message.value.into_owned(),
            reply(responder),
        ));
    }

    fn on_continue(&self, _message: Continue, _next: NextNotification<Continue>) {
        self.0.resume();
    }

    fn on_step(&self, message: Step, _next: NextNotification<Step>) {
        self.0.step(message.mode);
    }

    fn on_pause(&self, _message: Pause, _next: NextNotification<Pause>) {
        self.0.pause();
    }

    fn on_terminate(&self, _message: Terminate, _next: NextNotification<Terminate>) {
        self.0.terminate();
    }

    fn on_stack_trace(
        &self,
        _message: StackTrace,
        responder: Responder<Vec<StackFrame>, InspectionFailure>,
        _next: NextMessage<StackTrace>,
    ) {
        self.0.queue(Command::StackTrace(reply(responder)));
    }

    fn on_scopes(
        &self,
        message: Scopes,
        responder: Responder<Vec<Scope>, InspectionFailure>,
        _next: NextMessage<Scopes>,
    ) {
        self.0
            .queue(Command::Scopes(message.frame, reply(responder)));
    }

    fn on_variables(
        &self,
        message: Variables,
        responder: Responder<Vec<Variable>, InspectionFailure>,
        _next: NextMessage<Variables>,
    ) {
        self.0
            .queue(Command::Variables(message.reference, reply(responder)));
    }

    fn on_evaluate(
        &self,
        message: Evaluate<'static>,
        responder: Responder<Option<Evaluation>, EvaluationFailure>,
        _next: NextMessage<Evaluate<'static>>,
    ) {
        self.0.queue(Command::Evaluate(
            message.code.into_owned(),
            reply(responder),
        ));
    }
}
