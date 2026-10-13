//! Shared helpers for `greybel-interpreter` integration tests.

#![allow(dead_code)]

pub mod cases;
pub mod contexts;
pub mod debug;
pub mod greyscript;
pub mod value;

use greybel_documents::AsyncResourceProvider;
use greybel_interpreter::{Interpreter, RunResult, RuntimeError, Scheduler};
use greybel_test_support::future::block_on;
use greybel_test_support::interpreter::error_report;

/// Serializes printed lines as pretty JSON and appends the formatted runtime error when present.
pub fn snapshot(printed: &[String], error: Option<RuntimeError>) -> String {
    let mut snapshot = serde_json::to_string_pretty(printed).expect("json");

    if let Some(error) = error {
        snapshot.push('\n');
        snapshot.push_str(&error_report(&error));
    }

    snapshot
}

/// Starts the interpreter with breakpoints enabled and runs until the first breakpoint or completion.
pub fn pause<P: AsyncResourceProvider, S: Scheduler>(
    interpreter: &mut Interpreter<P, S>,
) -> RunResult {
    block_on(interpreter.start()).expect("starts");
    interpreter.vm_mut().expect("started").set_breakpoints(true);

    block_on(interpreter.resume()).expect("runs")
}
