//! Tests debugger breakpoints, stepping, pausing, and frame and variable inspection.

use greybel_interpreter::{RunResult, StepMode};
use greybel_test_support::future::block_on;

use crate::support::debug::{PROGRAM, debugged, line, paused_with_breakpoints, resume, uri};

#[test]
fn should_run_to_the_end_without_breakpoints() {
    let (mut interpreter, output, _) = debugged(PROGRAM);

    let result = resume(&mut interpreter);

    assert_eq!(result, RunResult::Complete);
    assert_eq!(*output.borrow(), ["2", "3"]);
}

#[test]
fn should_stop_at_a_lines_breakpoint() {
    let (mut interpreter, output, _) = paused_with_breakpoints(PROGRAM, &[7]);

    let result = resume(&mut interpreter);

    assert_eq!(line(&result), Some(7));
    assert!(output.borrow().is_empty());
}

#[test]
fn should_stop_at_a_breakpoint_inside_a_function() {
    let (mut interpreter, _, _) = paused_with_breakpoints(PROGRAM, &[2]);

    let result = resume(&mut interpreter);

    assert_eq!(line(&result), Some(2));
    assert_eq!(interpreter.vm().expect("started").depth(), 2);
}

#[test]
fn should_stop_at_the_first_statement_when_paused() {
    let (mut interpreter, _, control) = debugged(PROGRAM);

    control.pause();

    let result = resume(&mut interpreter);

    assert_eq!(line(&result), Some(1));
}

#[test]
fn should_step_over_a_call() {
    let (mut interpreter, _, control) = paused_with_breakpoints(PROGRAM, &[6]);

    resume(&mut interpreter);

    let depth = interpreter.vm().expect("started").depth();

    control.step(StepMode::Over, depth);

    let result = resume(&mut interpreter);

    assert_eq!(line(&result), Some(7));
}

#[test]
fn should_step_into_a_call() {
    let (mut interpreter, _, control) = paused_with_breakpoints(PROGRAM, &[6]);

    resume(&mut interpreter);
    control.set_breakpoints(&uri(&interpreter), []);

    control.step(StepMode::Into, interpreter.vm().expect("started").depth());

    let into = resume(&mut interpreter);

    assert_eq!(line(&into), Some(2));
}

#[test]
fn should_step_out_of_a_call() {
    let (mut interpreter, _, control) = paused_with_breakpoints(PROGRAM, &[6]);

    resume(&mut interpreter);
    control.set_breakpoints(&uri(&interpreter), []);
    control.step(StepMode::Into, interpreter.vm().expect("started").depth());
    resume(&mut interpreter);

    control.step(StepMode::Out, interpreter.vm().expect("started").depth());

    let out = resume(&mut interpreter);

    assert_eq!(line(&out), Some(7));
}

#[test]
fn should_show_the_frames_and_their_locals() {
    let (mut interpreter, _, _) = paused_with_breakpoints(PROGRAM, &[3]);

    resume(&mut interpreter);

    let vm = interpreter.vm().expect("started");

    let frames = vm.frames();
    let locals: Vec<(String, String)> = vm
        .frame_locals(0)
        .iter()
        .map(|(name, value)| {
            (
                vm.heap().display(name).to_string(),
                vm.heap().display(value).to_string(),
            )
        })
        .collect();

    let lines: Vec<_> = frames
        .iter()
        .map(|frame| frame.location.as_ref().map(|location| location.line))
        .collect();

    assert_eq!(lines, [Some(3), Some(6)]);
    assert_eq!(
        frames[0].start.as_ref().map(|location| location.line),
        Some(2)
    );
    assert_eq!(
        locals,
        [
            ("x".to_owned(), "1".to_owned()),
            ("y".to_owned(), "2".to_owned())
        ]
    );
}

#[test]
fn should_not_stop_inside_injected_code() {
    let (mut interpreter, output, _) = paused_with_breakpoints(PROGRAM, &[2, 7]);

    resume(&mut interpreter);

    block_on(interpreter.inject("print f(10)")).expect("injects");

    let result = resume(&mut interpreter);

    assert_eq!(*output.borrow(), ["20"]);
    assert_eq!(line(&result), Some(7));
}
