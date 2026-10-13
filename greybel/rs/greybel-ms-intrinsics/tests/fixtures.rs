//! Snapshot tests for every script in `tests/fixtures/scripts/` using the intrinsics. Update snapshots with
//! `INSTA_UPDATE=always cargo test`.

use greybel_interpreter::{InterpreterOptions, RuntimeErrorKind};
use greybel_ms_intrinsics::{API, IntrinsicError};
use greybel_test_support::interpreter::{
    constant_clock, interpreter_with_api, run_to_end, runtime_error,
};

use crate::support::run;

#[test]
fn should_run_every_script_fixture() {
    insta::glob!("fixtures/scripts/*.ms", |path| {
        let code = std::fs::read_to_string(path).expect("fixture");
        let printed = run(&code);
        insta::assert_snapshot!(printed);
    });
}

/// Sort-key lookup follows `__isa` like C# MiniScript; reference cycles fail with a depth limit instead of hanging
/// as in C++.
#[test]
fn should_fail_to_sort_by_a_key_through_a_reference_loop() {
    let code = "loop = {}\nloop.__isa = loop\n[loop, loop].sort(\"k\")";
    let (mut interpreter, _) =
        interpreter_with_api(code, InterpreterOptions::new(constant_clock()), API);
    let error = runtime_error(run_to_end(&mut interpreter).expect_err("the sort fails"));

    assert_eq!(error.kind, RuntimeErrorKind::IsaDepthLimit);
}

#[test]
fn should_type_the_intrinsics_errors() {
    let (mut interpreter, _) = interpreter_with_api(
        "[1, 2].remove(5)",
        InterpreterOptions::new(constant_clock()),
        API,
    );
    let error = runtime_error(run_to_end(&mut interpreter).expect_err("remove out of range fails"));

    assert_eq!(
        error.native::<IntrinsicError>(),
        Some(&IntrinsicError::IndexOutOfRange {
            index: 5,
            min: 0,
            max: 1
        })
    );
    assert_eq!(
        error.to_string(),
        "Index Error: index (5) out of range (0 to 1)"
    );
}
