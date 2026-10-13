//! Runs programs compiled with debugger support.

use std::rc::Rc;

use greybel_interpreter::{DebugControl, InterpreterOptions, RunResult};
use greybel_test_support::future::block_on;
use greybel_test_support::interpreter::CodeInterpreter;
use greybel_test_support::interpreter::test_api::interpreter_with;
use greybel_test_support::io::PrintLog;

pub const PROGRAM: &str =
    "f = function(x)\n  y = x * 2\n  return y\nend function\na = 1\nb = f(a)\nprint b\nprint a + b";

pub fn debugged(code: &str) -> (CodeInterpreter, PrintLog, Rc<DebugControl>) {
    let options = InterpreterOptions {
        debug: true,
        ..InterpreterOptions::new(std::rc::Rc::new(scheduler::ManualClock::constant()))
    };
    let (mut interpreter, output) = interpreter_with(code, options);

    block_on(interpreter.start()).expect("starts");

    let control = DebugControl::new();

    interpreter
        .vm_mut()
        .expect("started")
        .set_debug_control(Rc::clone(&control));

    (interpreter, output, control)
}

/// Starts `code` with debugger support, pauses before its first statement, and sets breakpoints on `lines`.
pub fn paused_with_breakpoints(
    code: &str,
    lines: &[u32],
) -> (CodeInterpreter, PrintLog, Rc<DebugControl>) {
    let (mut interpreter, output, control) = debugged(code);

    control.pause();
    resume(&mut interpreter);
    control.set_breakpoints(&uri(&interpreter), lines.iter().copied());

    (interpreter, output, control)
}

pub fn line(result: &RunResult) -> Option<u32> {
    match result {
        RunResult::Breakpoint(Some(location)) => Some(location.line),
        _ => None,
    }
}

pub fn resume(interpreter: &mut CodeInterpreter) -> RunResult {
    block_on(interpreter.resume()).expect("runs")
}

pub fn uri(interpreter: &CodeInterpreter) -> String {
    let frames = interpreter.vm().expect("started").frames();

    frames[0]
        .location
        .as_ref()
        .expect("a location")
        .uri
        .to_string()
}
