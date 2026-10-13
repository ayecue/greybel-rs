//! Tests for native calls that start child contexts.

use greybel_interpreter::RunResult;
use greybel_test_support::future::block_on;
use greybel_test_support::interpreter::resume_to_end;

use crate::support::contexts::{interpreter, printed};

#[test]
fn should_run_a_started_program_with_globals_of_its_own() {
    let code = "x = 1\nprint start(\"x = 2\nprint x\")\nprint x";

    assert_eq!(printed(code), ["2", "\"completion\"", "1"]);
}

/// A closure created in a child context uses the child's globals as its outer scope and the parent's globals as its
/// global scope when called by the parent.
#[test]
fn should_call_a_function_a_started_program_left() {
    let child = "y = 5\nshared.f = function\n  return [outer.y, globals.z]\nend function";
    let code = format!("z = 7\nstart(\"{child}\")\nprint shared.f");

    assert_eq!(printed(&code), ["[5, 7]"]);
}

#[test]
fn should_resume_the_starting_program_after_the_failure_of_the_started_one() {
    let code =
        "f = function\n  return start(\"\nmissing()\")\nend function\nprint f\nprint \"after\"";

    assert_eq!(printed(code), ["\"failure at 2 2 5\"", "\"after\""]);
}

/// Garbage collection while a child context is paused at a breakpoint retains its ancestor's values as GC roots.
#[test]
fn should_keep_the_values_of_the_waiting_program_through_a_collection() {
    let code = "m = {\"a\": [1, 2, 3]}\nstart(\"l = [4, 5]\nl = null\ndebugger\")\nprint m";
    let (mut interpreter, printed) = interpreter(code);

    block_on(interpreter.start()).expect("starts");
    block_on(interpreter.resume()).expect("runs");

    let freed = interpreter.vm_mut().expect("started").collect_garbage();

    resume_to_end(&mut interpreter).expect("runs");

    assert!(freed > 0);
    assert_eq!(printed.take(), ["{\"a\": [1, 2, 3]}"]);
}

#[test]
fn should_end_a_terminated_program_and_resume_the_one_that_started_it() {
    let code = "print start(\"debugger\nprint 1\")\nprint \"after\"";
    let (mut interpreter, printed) = interpreter(code);

    block_on(interpreter.start()).expect("starts");

    let stop = block_on(interpreter.resume()).expect("runs");

    assert!(matches!(stop, RunResult::Breakpoint(_)));

    let vm = interpreter.vm_mut().expect("started");

    assert_eq!(vm.context_depth(), 1);
    assert_eq!(vm.depth(), 2);
    vm.exit_contexts_n(1);
    resume_to_end(&mut interpreter).expect("runs");

    assert_eq!(printed.take(), ["\"termination\"", "\"after\""]);
}

#[test]
fn should_complete_once_every_terminated_program_ended() {
    let code = "print start(\"print start(\"\"debugger\"\")\")\nprint \"after\"";
    let (mut interpreter, printed) = interpreter(code);

    block_on(interpreter.start()).expect("starts");
    block_on(interpreter.resume()).expect("runs");
    interpreter.vm_mut().expect("started").exit_contexts_n(0);

    assert_eq!(
        block_on(interpreter.resume()).expect("runs"),
        RunResult::Complete
    );
    assert_eq!(printed.take(), Vec::<String>::new());
}
