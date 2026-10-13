//! Tests `#[native]` functions collected into `Api` constants.

use std::rc::Rc;

use greybel_interpreter::{Api, InterpreterOptions, Native, NativeDefault, ValueType, natives};
use greybel_test_support::future::block_on;
use greybel_test_support::interpreter::natives::{
    IO, add, append, async_list, deferred, deferred_exit, error_message, failure, first, greet,
    later, log, log_of, pairs, request_yield, run_and_log_with, size, take_yield_count, truthy,
};
use greybel_test_support::interpreter::test_api::{interpreter_for_code, interpreter_with};
use greybel_test_support::interpreter::{inline, run_to_end};
use greybel_test_support::io::{Recorder, ScriptedStdin};

#[test]
fn should_fill_in_a_default_number_argument() {
    let code = "log add(1, 2)\nlog add(1)";
    let log = log_of(code);

    assert_eq!(log, ["3", "11"]);
}

#[test]
fn should_fill_in_a_default_string_argument() {
    let code = "log greetUser(\"bob\")\nlog greetUser(\"bob\", \"?\")";
    let log = log_of(code);

    assert_eq!(log, ["hi bob!", "hi bob?"]);
}

#[test]
fn should_take_and_return_optional_values() {
    let code = "log maybe(2)\nlog maybe(null)";
    let log = log_of(code);

    assert_eq!(log, ["4", "null"]);
}

#[test]
fn should_fail_on_a_typed_argument_of_another_type() {
    let code = "greetUser(1)";
    let error = error_message(code);

    assert_eq!(error, "\"who\" must be a string.");
}

#[test]
fn should_fail_with_the_error_a_native_returns() {
    let code = "fail(\"broken\")";
    let error = error_message(code);

    assert_eq!(error, "broken");
}

/// A labelled native names itself in the trace of the errors it raises.
#[test]
fn should_name_a_labelled_native_in_stack_traces() {
    let code = "f = function\n  greetUser(1)\nend function\nf";
    let labelled = failure(code);

    assert_eq!(labelled.native_label, Some("greet"));
    assert_eq!(
        labelled.trace(),
        "at greet (native)\nat /unknown:2:3\nat /unknown:4:1"
    );
}

#[test]
fn should_leave_an_unlabelled_native_out_of_stack_traces() {
    let code = "fail(\"broken\")";
    let unlabelled = failure(code);

    assert_eq!(unlabelled.native_label, None);
    assert_eq!(unlabelled.trace(), "at /unknown:1:1");
}

#[test]
fn should_call_method_natives_as_type_intrinsics() {
    let code = "log {\"a\": 1, \"b\": 2}.size\nlog [5, 6].first\nlog [].first\nlog [].first(3)";
    let log = log_of(code);

    assert_eq!(log, ["2", "5", "-1", "3"]);
}

#[test]
fn should_not_find_method_natives_in_scope() {
    let code = "size";
    let error = error_message(code);

    assert_eq!(
        error,
        "Undefined Identifier: 'size' is unknown in this context"
    );
}

#[test]
fn should_await_async_natives() {
    let code = "log async(1) + 1";
    let log = log_of(code);

    assert_eq!(log, ["3"]);
}

/// A sequence a native returns is a list wherever its call is a value.
#[test]
fn should_make_a_list_of_a_sequence_used_as_a_value() {
    let code = "x = squares(4)\nx.push(9)\nlog x\nlog squares(3).len\nlog [squares(2)]";
    let log = log_of(code);

    assert_eq!(log, ["[0, 1, 4, 9, 9]", "3", "[[0, 1]]"]);
}

/// A `for` loop over a native's sequence takes items lazily and never builds the list.
#[test]
fn should_iterate_a_for_loops_sequence_without_making_its_list() {
    let body = "for s in squares(1000)\n  log s\n  if s == 4 then break\nend for";
    let codes = [
        body.to_owned(),
        format!("f = function\n{body}\nend function\nf"),
    ];

    for code in codes {
        take_yield_count();

        let log = log_of(&code);

        assert_eq!(log, ["0", "1", "4"]);
        assert_eq!(take_yield_count(), 3);
    }
}

/// The loop reads the sequence's items by index, which the body may set back.
#[test]
fn should_restart_a_sequence_when_the_loop_index_goes_back() {
    let body = "n = 0\nfor s in {}\n  log s\n  n = n + 1\n  if n == 3 then __s_idx = 0\nend for";
    let sequence = body.replace("{}", "squares(4)");
    let list = body.replace("{}", "[0, 1, 4, 9]");
    let iterated = log_of(&sequence);

    assert_eq!(iterated, log_of(&list));
    assert_eq!(iterated, ["0", "1", "4", "1", "4", "9"]);
}

/// An async native's sequence is a list even in a `for` loop.
#[test]
fn should_make_a_list_of_an_async_natives_sequence() {
    let code = "for s in asyncSquares(3)\n  log s\n  break\nend for";

    take_yield_count();

    let log = log_of(code);

    assert_eq!(log, ["0"]);
    assert_eq!(take_yield_count(), 3);
}

/// A call statement's sequence is dropped without yielding an item.
#[test]
fn should_drop_a_discarded_sequence() {
    let code = "squares(5)\nlog 1";

    take_yield_count();

    let log = log_of(code);

    assert_eq!(log, ["1"]);
    assert_eq!(take_yield_count(), 0);
}

#[test]
fn should_describe_a_plain_native() {
    let native: &Native = &add::NATIVE;
    let (label, is_method, is_async) = (native.label(), native.is_method(), native.is_async());
    let params = native.params();

    assert_eq!(label, None);
    assert!(!is_method && !is_async);
    assert_eq!(params[1].default(), NativeDefault::Number(10.0));
}

#[test]
fn should_describe_a_labelled_native() {
    let native: &Native = &greet::NATIVE;
    let (label, params) = (native.label(), native.params());

    assert_eq!(label, Some("greet"));
    assert_eq!(params[0].name(), "who");
}

#[test]
fn should_describe_a_method_native() {
    let native: &Native = &first::NATIVE;
    let is_method = native.is_method();

    assert!(is_method);
}

#[test]
fn should_describe_an_async_native() {
    let native: &Native = &later::NATIVE;
    let is_async = native.is_async();

    assert!(is_async);
}

#[test]
fn should_keep_a_native_callable_from_rust() {
    let (a, b) = (1.0, 2.0);
    let sum = add(a, b);

    assert!((sum - 3.0).abs() < f64::EPSILON);
}

#[test]
fn should_keep_an_async_native_callable_from_rust() {
    let value = 1.0;
    let next = block_on(later(value));

    assert_eq!(next, 2.0);
}

/// A native stored under several names or places is one function.
#[test]
fn should_make_a_native_listed_twice_one_function() {
    const TWICE: Api = Api::new()
        .set_globals(natives![log, size, "count" => size])
        .set_intrinsics(ValueType::Map, natives![size, "length" => size]);
    let code = "log @size == @{}.size\nlog @count == @size\nlog @{}.length == @size";

    let (log, result) = run_and_log_with(code, TWICE);

    result.expect("runs");
    assert_eq!(log, ["1", "1", "1"]);
}

#[test]
fn should_read_and_write_through_the_hosts_stdin_and_stdout() {
    let stdin = ScriptedStdin::new(&["bob", "secret"]);
    let options = InterpreterOptions {
        stdin: Rc::new(stdin),
        ..InterpreterOptions::new(std::rc::Rc::new(scheduler::ManualClock::constant()))
    };
    let (interpreter, output) = interpreter_with(
        "write userInput(\"name: \")\nwrite userInput(\"pw: \", true)\nwrite userKey",
        options,
    );
    let mut interpreter = interpreter.with_api(IO);

    run_to_end(&mut interpreter).expect("runs");
    assert_eq!(*output.borrow(), ["name: bob", "pw: secret*", "Enter"]);
}

#[test]
fn should_flush_stdout_before_waiting_where_it_yields_and_at_the_end() {
    let recorder = Recorder::default();
    let options = InterpreterOptions {
        stdout: Rc::new(recorder.clone()),
        stdin: Rc::new(recorder.clone()),
        ..InterpreterOptions::new(std::rc::Rc::new(scheduler::ManualClock::constant()))
    };
    let mut interpreter = greybel_interpreter::Interpreter::new(
        options,
        inline("write \"a\"\nuserInput\nwrite \"b\"\nendSlice\nwrite \"c\""),
    )
    .with_api(IO)
    .with_api(Api::new().set_globals(natives!["endSlice" => request_yield]));

    run_to_end(&mut interpreter).expect("runs");
    assert_eq!(
        recorder.calls(),
        [
            "print a",
            "flush",
            "read",
            "print b",
            "flush if due",
            "print c",
            "flush"
        ]
    );
}

#[test]
fn should_pass_bool_arguments_by_miniscript_truthiness() {
    const TRUTHY: Api = Api::new().set_globals(natives![log, truthy]);
    let code =
        "for v in [0, 1, \"\", \"a\", [], [0], {}, {1: 1}, null, @log]\nlog truthy(@v)\nend for";

    let (log, result) = run_and_log_with(code, TRUTHY);

    result.expect("runs");
    assert_eq!(log, ["0", "1", "0", "1", "0", "1", "0", "1", "0", "1"]);
}

/// Natives take list and map contents, change lists in place, and return Rust collections as new lists and maps.
#[test]
fn should_take_and_return_contents() {
    const CONTENTS: Api = Api::new()
        .set_globals(natives![log, pairs, "asyncList" => async_list])
        .set_intrinsics(ValueType::List, natives![append, first])
        .set_intrinsics(ValueType::Map, natives![size]);
    let code = "m = pairs(3)\nlog m\nlog m.size\nl = [1]\nlog l.append(2)\nlog l\nlog l.first\nlog asyncList(3)";

    let (log, result) = run_and_log_with(code, CONTENTS);

    result.expect("runs");
    assert_eq!(
        log,
        [
            r#"{"k0": [], "k1": [1], "k2": [2, 2]}"#,
            "3",
            "2",
            "[1, 2]",
            "1",
            "[0, 1, 2]",
        ]
    );
}

#[test]
fn should_fail_on_contents_of_another_type() {
    const CONTENTS: Api = Api::new()
        .set_globals(natives![log])
        .set_intrinsics(ValueType::String, natives![append]);
    let (interpreter, _) = interpreter_for_code("\"a\".append(1)");
    let mut interpreter = interpreter.with_api(CONTENTS);
    let error = run_to_end(&mut interpreter).expect_err("fails");

    assert_eq!(error.to_string(), "\"self\" must be a list.");
}

/// A native whose result ends the program, made on the VM, ends it before the next statement.
#[test]
fn should_end_the_program_when_an_async_natives_result_exits() {
    const EXITING: Api = Api::new().set_globals(natives![log, deferred_exit]);

    let (log, result) = run_and_log_with("log 1\nlog deferred_exit\nlog 2", EXITING);

    result.expect("runs");
    assert_eq!(log, ["1"]);
}

/// A native that runs synchronously may return a future: the program waits for it, and its result is made on the VM.
#[test]
fn should_finish_a_deferred_native_on_the_vm() {
    const DEFERRED: Api = Api::new().set_globals(natives![log, deferred]);
    let code =
        "log deferred(2)\nf = function\n  return deferred(3)\nend function\nlog f\ndeferred(4)";

    let (log, result) = run_and_log_with(code, DEFERRED);

    result.expect("runs");
    assert_eq!(log, ["[2, 1]", "[3, 2]"]);
}
