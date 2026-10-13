//! Shared helpers for `greybel-ms-intrinsics` integration tests.

#![allow(dead_code)]

use greybel_ms_intrinsics::API;
use greybel_test_support::interpreter::{
    constant_clock_options, error_report, interpreter_with_api, run_to_end, runtime_error_of,
};
use mono_random::Random;

/// Runs `code` and returns its printed lines, followed by the formatted runtime error when execution fails.
pub fn run(code: &str) -> String {
    let (interpreter, printed) = interpreter_with_api(code, constant_clock_options(), API);

    let mut interpreter = interpreter.with_host_data(Random::new(0));
    let error = runtime_error_of(run_to_end(&mut interpreter));
    let mut snapshot = printed.borrow().join("\n");

    if let Some(error) = error {
        snapshot.push('\n');
        snapshot.push_str(&error_report(&error));
    }

    snapshot
}
