//! Interpreter test harnesses with deterministic clocks, captured output, the reference test API, and `#[native]`
//! examples.

pub mod natives;
pub mod test_api;

use std::rc::Rc;

use greybel_documents::AsyncResourceProvider;
use greybel_documents_memory_provider::MemoryProvider;
use greybel_interpreter::{
    Clock, InstructionBudget, Interpreter, InterpreterError, InterpreterOptions, Register,
    RunResult, RuntimeError, Scheduler, Stdout,
};
use scheduler::ManualClock;

use crate::future::block_on;
use crate::io::{CapturedStdout, MarkedStdout, PrintLog};

/// Creates a zero-fixed manual clock with immediate sleeps, making `time`, `wait`, and world time deterministic.
#[must_use]
pub fn constant_clock() -> Rc<dyn Clock> {
    Rc::new(ManualClock::constant())
}

/// Creates default interpreter options with [`constant_clock`].
#[must_use]
pub fn constant_clock_options() -> InterpreterOptions {
    InterpreterOptions::new(constant_clock())
}

/// Creates marked stdout and deterministic options for output snapshots.
#[must_use]
pub fn snapshot_options() -> (Rc<MarkedStdout>, InterpreterOptions) {
    let stdout = Rc::new(MarkedStdout::default());
    let options = InterpreterOptions {
        stdout: Rc::clone(&stdout) as Rc<dyn Stdout>,
        ..constant_clock_options()
    };

    (stdout, options)
}

/// Creates shared print_log lines and configures `options` to capture them through [`CapturedStdout`].
#[must_use]
pub fn with_captured_stdout(options: InterpreterOptions) -> (PrintLog, InterpreterOptions) {
    let print_log = PrintLog::default();
    let options = InterpreterOptions {
        stdout: Rc::new(CapturedStdout(Rc::clone(&print_log))),
        ..options
    };

    (print_log, options)
}

/// Target name for inline source.
pub const INLINE: &str = "unknown";

/// Creates a memory provider with `code` stored as [`INLINE`].
#[must_use]
pub fn inline(code: &str) -> MemoryProvider {
    [(INLINE, code)].into_iter().collect()
}

/// Interpreter type for in-memory inline source.
pub type CodeInterpreter<S = InstructionBudget> = Interpreter<MemoryProvider, S>;

/// Creates an inline interpreter using `api`, captured stdout, and `options`, with target forced to [`INLINE`].
pub fn interpreter_with_api(
    code: &str,
    options: InterpreterOptions,
    api: impl Register + 'static,
) -> (CodeInterpreter, PrintLog) {
    let (print_log, options) = with_captured_stdout(InterpreterOptions {
        target: INLINE.to_owned(),
        ..options
    });
    let interpreter = Interpreter::new(options, inline(code)).with_api(api);

    (interpreter, print_log)
}

/// Resumes a started program to completion and disables breakpoints after its first stop.
pub fn resume_to_end<P: AsyncResourceProvider, S: Scheduler>(
    interpreter: &mut Interpreter<P, S>,
) -> Result<(), InterpreterError> {
    loop {
        match block_on(interpreter.resume())? {
            RunResult::Complete => return Ok(()),
            RunResult::Breakpoint(_) => interpreter
                .vm_mut()
                .expect("started")
                .set_breakpoints(false),
        }
    }
}

/// Starts and completes a program while ignoring breakpoints.
pub fn run_to_end<P: AsyncResourceProvider, S: Scheduler>(
    interpreter: &mut Interpreter<P, S>,
) -> Result<(), InterpreterError> {
    block_on(interpreter.start())?;

    resume_to_end(interpreter)
}

/// Extracts `RuntimeError` from `error`; all other `InterpreterError` variants panic.
#[must_use]
pub fn runtime_error(error: InterpreterError) -> RuntimeError {
    match error {
        InterpreterError::Runtime(error) => error,
        other => panic!("{other}"),
    }
}

/// Maps runtime errors to `Some` and success to `None`; non-runtime interpreter errors panic.
#[must_use]
pub fn runtime_error_of(result: Result<(), InterpreterError>) -> Option<RuntimeError> {
    result.err().map(runtime_error)
}

/// Formats `error` as `error: <message>` plus its stack trace.
#[must_use]
pub fn error_report(error: &RuntimeError) -> String {
    format!("error: {error}\n{}", error.trace())
}
