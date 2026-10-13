//! The reference's specific cases: exiting, stack traces and errors.

use std::sync::{Arc, Mutex};
use std::task::Poll;

use greybel_core::{Dialect, PositionEncoding};
use greybel_interpreter::bytecode::{
    Code, Destination, Op, Operand, VariableDestination, VariableOperand,
};
use greybel_interpreter::{InstructionBudget, InterpreterOptions, RunResult, Scheduler, Value, Vm};
use greybel_test_support::future::block_on;
use greybel_test_support::interpreter::test_api::{
    interpreter_for_code, interpreter_with, program_for_code, run_code, run_code_as,
};
use greybel_test_support::interpreter::{
    constant_clock_options, resume_to_end, run_to_end, runtime_error,
};

use crate::support::cases::{
    Counting, FOLDED, GARBAGE, OneAtATime, PAUSED_FRAME, Shared, Slices, awaiting, enter_call,
    error, fetching, has_slots, poll_once, printed, prompting, run_with_encoding, started,
};
use crate::support::pause;

#[test]
fn should_stop_the_program_on_exit() {
    let code = "test = \"foo\"\nexit\nprint(\"123\")\nprint(test)";

    let printed = printed(code);

    assert!(printed.is_empty());
}

#[test]
fn should_exit_while_resolving_an_argument() {
    let code = "f = function; exit(\"bye\"); end function\nu = function(param); print \"f\"; end function\nu(f)";

    let printed = printed(code);

    assert_eq!(printed, ["\"bye\""]);
}

#[test]
fn should_exit_while_resolving_later_arguments() {
    let code = r#"
        a = function; print "a"; end function
        b = function; print "b"; end function
        c = function; print "c"; end function
        d = function; print "d"; end function
        f = function; exit("bye"); end function
        u = function(a, b, c, d, e, f); print "f"; end function
        u(a, b, f, c, d)
    "#;

    let printed = printed(code);

    assert_eq!(printed, ["\"a\"", "\"b\"", "\"bye\""]);
}

#[test]
fn should_negate_a_number_as_a_fuzzy_truth_value() {
    let code = "print not 0.25\nprint not -2\nprint not 0\nprint not \"\"\nprint not \"a\"\nprint not [1]\nprint not null";

    let printed = printed(code);

    assert_eq!(printed, ["0.75", "0", "1", "1", "0", "0", "1"]);
}

/// MiniScript's `self` is null in a function called without one, and so is any member read from it.
#[test]
fn should_read_self_as_null_in_a_function_called_without_one() {
    let code = "C = {\"n\": 2}\nC.m = function(a)\n  return [self, self.n, self[\"n\"], a]\nend function\nc = new C\nf = @c.m\nprint f(1)\nprint c.m(1)[1:]\nprint self";

    let printed = printed(code);

    assert_eq!(printed, ["[null, null, null, 1]", "[2, 2, 1]", "null"]);
}

#[test]
fn should_exit_in_a_condition() {
    let code = "if not exit(\"bye\") then\n  print \"hey\"\nend if";

    let printed = printed(code);

    assert_eq!(printed, ["\"bye\""]);
}

#[test]
fn should_exit_in_an_else_if_condition() {
    let code = r#"
        if false then
          print "not here"
        else if exit("bye") then
          print "not here"
        else
          print "hey"
        end if
        print "hey"
    "#;

    let printed = printed(code);

    assert_eq!(printed, ["\"bye\""]);
}

#[test]
fn should_carry_the_call_stack_in_errors() {
    let code = r#"
            foo = function
              unknown.test = "wrong"
            end function

            bar = function
              a = 1
              b = 2
              foo
            end function

            while (bar)

            end while
          "#;
    let (_, error) = run_code(code);
    let error = error.expect("fails");

    assert_eq!(
        error.to_string(),
        "Undefined Identifier: 'unknown' is unknown in this context"
    );
    assert_eq!(
        error.trace(),
        "at /unknown:3:15\nat /unknown:9:15\nat /unknown:12:20"
    );
}

#[test]
fn should_fail_to_look_up_members_of_null() {
    let expected = "Type Error (while attempting to look up push)";
    let codes = [
        "test = {}\ntest.item = null\ntest.item.push(234)",
        "test = {}\ntest.item = null\ntest.item.push",
        "test = {}\ntest.item = null\nfoo = new test\nfoo.bar = function\n  super.item.push(123)\nend function\nfoo.bar",
        "test = {}\ntest.item = null\nfoo = new test\nfoo.bar = function\n  super.item.push\nend function\nfoo.bar",
    ];
    let errors = codes.map(error);

    for (code, error) in codes.into_iter().zip(errors) {
        assert_eq!(error, expected, "{code}");
    }
}

#[test]
fn should_let_nested_functions_see_only_their_outer_scope() {
    let code = r#"
          a = function
            foo = {"bar": 123}
            b = function
              c = function
                print foo.bar
              end function
              c
            end function
            b
          end function
          a
        "#;
    let error = error(code);

    assert_eq!(
        error,
        "Undefined Identifier: 'foo' is unknown in this context"
    );
}

#[test]
fn should_give_function_arguments_no_outer_scope() {
    let code = r#"
          someFunc = function(fn)
            fn
          end function

          main = function()
              a = function(x)
                someFunc function
                  print x
                end function
              end function
              a("wa")
          end function

          main
        "#;
    let error = error(code);

    assert_eq!(
        error,
        "Undefined Identifier: 'x' is unknown in this context"
    );
}

/// MiniScript's loop reads null's length as null, which ends the loop at once.
#[test]
fn should_not_iterate_over_null() {
    let code = "x = null\nfor i in x\n  print i\nend for\nprint 1";

    let printed = printed(code);

    assert_eq!(printed, ["1"]);
}

#[test]
fn should_return_a_value_on_calling_it() {
    let code = "x = 5\nprint x()";

    let printed = printed(code);

    assert_eq!(printed, ["5"]);
}

#[test]
fn should_reject_arguments_to_a_called_value() {
    let code = "x = 5\nprint x(1)";
    let error = error(code);

    assert_eq!(error, "Too Many Arguments");
}

#[test]
fn should_run_in_slices_and_yield() {
    let (interpreter, output) =
        interpreter_for_code("for i in range(1, 200)\nend for\nprint \"done\"");
    let mut interpreter = interpreter.with_scheduler(Slices::default());
    let result = block_on(interpreter.run()).expect("runs");

    assert_eq!(result, RunResult::Complete);
    assert!(interpreter.scheduler().yields > 10);
    assert_eq!(*output.borrow(), ["\"done\""]);
    assert_eq!(interpreter.vm().expect("started").open_values(), 0);
}

#[test]
fn should_stop_at_breakpoints() {
    let (mut interpreter, output) = interpreter_for_code("print 1\ndebugger\nprint 2");
    let result = block_on(interpreter.run()).expect("runs");
    let RunResult::Breakpoint(Some(location)) = result else {
        panic!("expected a breakpoint");
    };

    assert_eq!((location.line, location.character), (2, 1));
    assert_eq!(*output.borrow(), ["1"]);
}

#[test]
fn should_resume_after_a_breakpoint() {
    let (mut interpreter, output) = interpreter_for_code("print 1\ndebugger\nprint 2");

    block_on(interpreter.run()).expect("runs");

    resume_to_end(&mut interpreter).expect("runs");

    assert_eq!(*output.borrow(), ["1", "2"]);
}

/// An async native's future may resolve long after the call; a run dropped while waiting resumes at the call.
#[test]
fn should_wait_for_a_native_that_responds_later() {
    let (mut interpreter, _output, _response) =
        prompting("x = prompt(\"name\")\nprint \"hi \" + x");

    let poll = poll_once(&mut interpreter);

    assert!(poll.is_pending());
}

#[test]
fn should_still_wait_for_a_response_when_polled_again() {
    let (mut interpreter, output, _response) = prompting("x = prompt(\"name\")\nprint \"hi \" + x");
    let _ = poll_once(&mut interpreter);

    let poll = poll_once(&mut interpreter);

    assert!(poll.is_pending(), "waits for the response");
    assert!(output.borrow().is_empty());
}

#[test]
fn should_resume_at_the_call_once_a_native_responds() {
    let (mut interpreter, output, response) = prompting("x = prompt(\"name\")\nprint \"hi \" + x");
    let _ = poll_once(&mut interpreter);
    let _ = poll_once(&mut interpreter);

    *response.borrow_mut() = Some("bob".into());

    let result = block_on(interpreter.resume()).expect("runs");

    assert_eq!(result, RunResult::Complete);
    assert_eq!(*output.borrow(), ["\"hi bob\""]);
}

/// Tests C#-compatible closure scoping for directly assigned function literals.
#[test]
fn should_capture_the_outer_scope_of_an_assigned_function_literal() {
    let assigned = r#"
        make = function(x)
          g = function()
            return x
          end function
          return @g
        end function
        f = make(1)
        print f
    "#;

    let printed = printed(assigned);

    assert_eq!(printed, ["1"]);
}

#[test]
fn should_not_capture_the_outer_scope_of_a_returned_literal() {
    let returned = r#"
        make = function(x)
          return function()
            return x
          end function
        end function
        f = make(3)
        print f
    "#;
    let error = error(returned);

    assert_eq!(
        error,
        "Undefined Identifier: 'x' is unknown in this context"
    );
}

#[test]
fn should_not_capture_the_outer_scope_of_a_passed_literal() {
    let passed = r#"
        store = {}
        keep = function(f)
          store.f = @f
        end function
        make = function(x)
          keep function()
            return x
          end function
        end function
        make(4)
        print store.f
    "#;
    let error = error(passed);

    assert_eq!(
        error,
        "Undefined Identifier: 'x' is unknown in this context"
    );
}

#[test]
fn should_let_a_returned_literal_fall_back_to_the_globals() {
    let falls_back_to_globals = r#"
        x = "global"
        make = function(x)
          return function()
            return x
          end function
        end function
        f = make("local")
        print f
    "#;

    let printed = printed(falls_back_to_globals);

    assert_eq!(printed, ["\"global\""]);
}

#[test]
fn should_run_a_program_making_garbage() {
    let (mut interpreter, output) = interpreter_for_code(GARBAGE);

    run_to_end(&mut interpreter).expect("runs");
    assert_eq!(*output.borrow(), ["40001"]);
}

#[test]
fn should_collect_the_garbage() {
    let (mut interpreter, _) = interpreter_for_code(GARBAGE);

    run_to_end(&mut interpreter).expect("runs");

    let vm = interpreter.vm_mut().expect("started");

    vm.collect_garbage();
    assert!(
        vm.heap().all_objects_len() < 200,
        "{}",
        vm.heap().all_objects_len()
    );
}

#[test]
fn should_keep_pinned_values_through_collections() {
    let (mut interpreter, _) = started("x = 1");
    let vm = interpreter.vm_mut().expect("started");
    let list = vm.heap_mut().add_list(vec![Value::from("kept")]);
    let pin = vm.pin(Value::List(list));

    vm.collect_garbage();
    assert_eq!(
        vm.heap().display(vm.pinned_value(&pin)).to_string(),
        "[\"kept\"]"
    );
}

#[test]
fn should_give_back_the_pinned_value_on_unpin() {
    let (mut interpreter, _) = started("x = 1");
    let vm = interpreter.vm_mut().expect("started");
    let list = vm.heap_mut().add_list(vec![Value::from("kept")]);
    let pin = vm.pin(Value::List(list));

    vm.collect_garbage();

    let value = vm.unpin(pin);

    assert!(matches!(value, Value::List(_)));
}

#[test]
fn should_collect_unpinned_values() {
    let (mut interpreter, _) = started("x = 1");
    let vm = interpreter.vm_mut().expect("started");
    let list = vm.heap_mut().add_list(vec![Value::from("kept")]);
    let pin = vm.pin(Value::List(list));

    vm.collect_garbage();

    let _value = vm.unpin(pin);

    vm.collect_garbage();

    let collected = vm.collect_garbage();

    assert_eq!(collected, 0);
}

/// A function reading an API name keeps reading it from the API until a nearer scope gains that name.
#[test]
fn should_read_a_name_from_the_nearest_scope_once_one_gains_it() {
    let code = r#"
        f = function(x)
          return sqrt(x)
        end function
        results = []
        for i in range(1, 4)
          results.push f(i * i)
          if i == 2 then
            sqrt = function(x)
              return -x
            end function
          end if
        end for
        print results
        make = function
          inner = function
            return y
          end function
          r = [inner]
          y = "outer"
          r.push inner
          return r
        end function
        y = "global"
        print make
    "#;

    let printed = printed(code);

    assert_eq!(printed, ["[1, 2, -9, -16]", "[\"global\", \"outer\"]"]);
}

/// A removed and reassigned name is read at its new entry, not at the entry another key took over.
#[test]
fn should_read_a_reassigned_name_at_its_new_entry() {
    let code = r#"
        z = 1
        g = function
          return z
        end function
        seen = [g]
        remove(globals, "z")
        globals.w = 5
        z = 2
        seen.push g
        print seen
    "#;

    let printed = printed(code);

    assert_eq!(printed, ["[1, 2]"]);
}

/// A statement that only reads or calls drops its result, so loops of them don't pile values on the VM's stack.
#[test]
fn should_drop_the_results_of_call_statements() {
    let statements = ["f", "xs.len", "xs[0]"];
    let last: Vec<Op> = statements
        .iter()
        .map(|statement| {
            let program = program_for_code(statement);
            let ops = &program.code(program.entry()).ops;

            ops.last().expect("an op").clone()
        })
        .collect();

    assert!(
        matches!(
            last.as_slice(),
            [
                Op::GetVariable { discard: true, .. },
                Op::GetMember { discard: true, .. },
                Op::GetProperty { discard: true, .. },
            ]
        ),
        "{last:?}"
    );
}

/// A list growing in place counts towards the next collection, though it is one object.
#[test]
fn should_count_list_growth_towards_the_next_collection() {
    let (mut interpreter, _) = started("x = 1");
    let vm = interpreter.vm_mut().expect("started");
    let list = vm.heap_mut().add_list(Vec::new());
    let before = vm.heap().last_allocation_size();

    list.items_mut(vm.heap_mut())
        .extend((0..1000).map(|n| Value::from(f64::from(n))));

    let grown = vm.heap().last_allocation_size() - before;

    assert!(grown >= 1000 * size_of::<Value>(), "{grown}");
}

#[test]
fn should_count_map_growth_towards_the_next_collection() {
    let (mut interpreter, _) = started("x = 1");
    let vm = interpreter.vm_mut().expect("started");
    let map = vm.heap_mut().add_map();
    let before = vm.heap().last_allocation_size();

    for key in 0..1000 {
        map.set(vm.heap_mut(), Value::from(f64::from(key)), Value::Null);
    }

    let grown = vm.heap().last_allocation_size() - before;

    assert!(grown >= 1000 * 2 * size_of::<Value>(), "{grown}");
}

/// Lists grown to a thousand items each are collected as their bytes add up, not once thousands of objects exist.
#[test]
fn should_collect_lists_grown_in_place() {
    let code = r#"
        i = 0
        while i < 200
          xs = []
          j = 0
          while j < 1000
            xs.push(j)
            j = j + 1
          end while
          i = i + 1
        end while
    "#;
    let live_after = |code: &str| {
        let options = InterpreterOptions {
            collection_threshold: 64 << 10,
            ..InterpreterOptions::new(std::rc::Rc::new(scheduler::ManualClock::constant()))
        };
        let (mut interpreter, _) = interpreter_with(code, options);

        run_to_end(&mut interpreter).expect("runs");

        interpreter.vm().expect("ran").heap().all_objects_len()
    };
    let baseline = live_after("xs = []");
    let live = live_after(code);

    assert!(live - baseline < 10, "{live} live, {baseline} at baseline");
}

/// After a collection the heap may grow by as much as stays live, so a large live set doesn't trigger a collection every few allocations.
#[test]
fn should_measure_the_live_bytes_on_collection() {
    let code = "keep = range(1, 10000)";
    let (mut interpreter, _) = interpreter_for_code(code);

    run_to_end(&mut interpreter).expect("runs");

    let vm = interpreter.vm_mut().expect("ran");

    vm.collect_garbage();
    assert!(vm.heap().current_allocation_size() >= 10000 * size_of::<Value>());
    assert_eq!(vm.heap().last_allocation_size(), 0);
}

/// Strings are indexed, sliced and walked in UTF-16 code units, as in C# MiniScript; a lone half of a surrogate pair reads as U+FFFD.
#[test]
fn should_count_string_positions_in_utf16_units() {
    let code = r#"
        text = "aé😀b"
        print text.len
        print text[1]
        print text[2]
        print text[2:4]
        print text[-1]
        print text[1:]
        count = 0
        for c in text
          count = count + 1
        end for
        print count
        print "😀" * 1.5
    "#;

    let printed = printed(code);

    assert_eq!(
        printed,
        [
            "5",
            "\"é\"",
            "\"\u{FFFD}\"",
            "\"😀\"",
            "\"b\"",
            "\"é😀b\"",
            "5",
            "\"😀\u{FFFD}\"",
        ]
    );
}

/// Code injected at a breakpoint runs in the paused frame's scope, then the program continues where it stopped.
#[test]
fn should_pause_in_a_function_at_its_breakpoint() {
    let (mut interpreter, _) = interpreter_for_code(PAUSED_FRAME);

    let result = pause(&mut interpreter);

    assert!(matches!(result, RunResult::Breakpoint(_)));
}

#[test]
fn should_run_injected_code_in_the_paused_frame() {
    let (mut interpreter, output) = interpreter_for_code(PAUSED_FRAME);

    pause(&mut interpreter);
    block_on(interpreter.inject("print x + y\ny = 100\ng = function\n  return 7\nend function"))
        .expect("injects");
    assert_eq!(*output.borrow(), ["15"]);
}

#[test]
fn should_continue_where_it_stopped_after_injections() {
    let (mut interpreter, output) = interpreter_for_code(PAUSED_FRAME);

    pause(&mut interpreter);
    block_on(interpreter.inject("print x + y\ny = 100\ng = function\n  return 7\nend function"))
        .expect("injects");
    block_on(interpreter.inject("print g")).expect("injects");

    resume_to_end(&mut interpreter).expect("runs");

    assert_eq!(*output.borrow(), ["15", "7", "100"]);
}

#[test]
fn should_pause_at_a_debugger_statement() {
    let (mut interpreter, _) = interpreter_for_code("a = 1\ndebugger\nprint a");

    let result = pause(&mut interpreter);

    assert!(matches!(result, RunResult::Breakpoint(_)));
}

#[test]
fn should_report_the_error_of_a_failing_injection() {
    let (mut interpreter, _) = interpreter_for_code("a = 1\ndebugger\nprint a");

    pause(&mut interpreter);

    let result = block_on(interpreter.inject("print missing"));
    let error = result.expect_err("fails");

    assert_eq!(
        error.to_string(),
        "Undefined Identifier: 'missing' is unknown in this context"
    );
}

#[test]
fn should_leave_the_program_paused_after_a_failing_injection() {
    let (mut interpreter, output) = interpreter_for_code("a = 1\ndebugger\nprint a");

    pause(&mut interpreter);

    let _ = block_on(interpreter.inject("print missing"));
    let result = block_on(interpreter.resume()).expect("runs");

    assert_eq!(result, RunResult::Complete);
    assert_eq!(*output.borrow(), ["1"]);
}

#[test]
fn should_refuse_to_inject_into_a_finished_program() {
    let (mut interpreter, _) = interpreter_for_code("a = 1\ndebugger\nprint a");

    pause(&mut interpreter);

    let _ = block_on(interpreter.inject("print missing"));

    block_on(interpreter.resume()).expect("runs");

    let result = block_on(interpreter.inject("print 1"));
    let error = result.expect_err("finished");

    assert_eq!(error.to_string(), "Unable to inject into last context.");
}

#[test]
fn should_pause_before_injecting_code_that_awaits() {
    let (mut interpreter, _) = fetching("debugger\nprint 2");

    let result = pause(&mut interpreter);

    assert!(matches!(result, RunResult::Breakpoint(_)));
}

/// Injected code awaits async natives like the program does.
#[test]
fn should_await_async_natives_in_injected_code() {
    let (mut interpreter, output) = fetching("debugger\nprint 2");

    pause(&mut interpreter);
    block_on(interpreter.inject("print fetch")).expect("injects");

    let result = block_on(interpreter.resume()).expect("runs");

    assert_eq!(result, RunResult::Complete);
    assert_eq!(*output.borrow(), ["1", "2"]);
}

/// An async native's future is awaited, ready or pending, and an error from it ends the program with the stack trace at the call.
#[test]
fn should_await_async_natives() {
    let (mut interpreter, output) = awaiting();
    let error = runtime_error(block_on(interpreter.run()).expect_err("fails"));

    assert_eq!(*output.borrow(), ["2", "41"]);
    assert_eq!(error.to_string(), "offline");
    assert_eq!(error.stack.len(), 2, "{}", error.trace());
    assert!(interpreter.vm().expect("started").is_finished());
}

#[test]
fn should_finish_on_resuming_after_an_async_native_failed() {
    let (mut interpreter, _) = awaiting();
    let _ = block_on(interpreter.run()).expect_err("fails");
    let result = block_on(interpreter.resume()).expect("runs");

    assert_eq!(result, RunResult::Complete);
}

/// A run waits for a native's future until it's woken, here from another thread.
#[test]
fn should_wait_for_native_futures_to_wake_the_run() {
    let (interpreter, output) = interpreter_for_code("print fetch + 1");
    let shared = Arc::new(Mutex::new(Shared::default()));
    let state = Arc::clone(&shared);
    let mut interpreter = interpreter.with_api(move |vm: &mut Vm| {
        let state = Arc::clone(&state);
        let fetch = vm.add_async_native([], move |_, _| {
            let state = Arc::clone(&state);

            std::future::poll_fn(move |context| {
                let mut state = state.lock().expect("lock");

                match state.value {
                    Some(value) => Poll::Ready(Ok(Value::from(value))),
                    None => {
                        state.waker = Some(context.waker().clone());

                        Poll::Pending
                    }
                }
            })
        });

        vm.define("fetch", fetch);
    });
    let worker = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(20));

        let mut state = shared.lock().expect("lock");

        state.value = Some(41.0);
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    });
    let result = block_on(interpreter.run()).expect("runs");

    worker.join().expect("joins");
    assert_eq!(result, RunResult::Complete);
    assert_eq!(*output.borrow(), ["42"]);
}

/// `InterpreterOptions::string_encoding` picks the units string positions count.
#[test]
fn should_count_string_positions_in_the_configured_encoding() {
    let code = r#"
        text = "aé😀b"
        print text.len
        print text[1]
        print text[-1]
        count = 0
        for c in text
          count = count + 1
        end for
        print count
    "#;
    let encodings = [
        PositionEncoding::Utf8,
        PositionEncoding::Utf16,
        PositionEncoding::Utf32,
    ];
    let [utf8, utf16, utf32] = encodings.map(|encoding| run_with_encoding(code, encoding));

    assert_eq!(utf8, ["8", "\"\u{FFFD}\"", "\"b\"", "8"]);
    assert_eq!(utf16, ["5", "\"é\"", "\"b\"", "5"]);
    assert_eq!(utf32, ["4", "\"é\"", "\"b\"", "4"]);
}

/// The scheduler yields every 2,048 instructions, counting the yields and the slices' sizes.
#[test]
fn should_yield_when_the_scheduler_says() {
    let (interpreter, output) =
        interpreter_for_code("i = 0\nwhile i < 20000\n  i = i + 1\nend while\nprint i");
    let mut interpreter = interpreter.with_scheduler(Counting::default());
    let result = block_on(interpreter.run()).expect("runs");

    assert_eq!(result, RunResult::Complete);
    assert_eq!(*output.borrow(), ["20000"]);

    let slices = &interpreter.scheduler().slices;

    assert!(slices.len() >= 5, "{slices:?}");
    assert!(slices.iter().all(|&slice| slice == 2048));
}

#[test]
fn should_yield_once_the_instruction_budget_is_spent() {
    let mut budget = InstructionBudget::new(3000);
    let decisions = [1024; 4].map(|executed| budget.should_yield(executed));

    assert!(!decisions[0]);
    assert!(!decisions[1]);
    assert!(decisions[2]);
    assert!(!decisions[3]);
}

/// `yielding` keeps the scheduler's decisions and also awaits the given future at every yield, e.g. a page's event loop.
#[test]
fn should_await_the_given_future_at_every_yield() {
    let yields = std::rc::Rc::new(std::cell::Cell::new(0));
    let scheduler = InstructionBudget::new(2048).with_yield_hook({
        let yields = std::rc::Rc::clone(&yields);

        move || {
            yields.set(yields.get() + 1);

            std::future::ready(())
        }
    });
    let (interpreter, output) =
        interpreter_for_code("i = 0\nwhile i < 20000\n  i = i + 1\nend while\nprint i");
    let mut interpreter = interpreter.with_scheduler(scheduler);
    let result = block_on(interpreter.run()).expect("runs");

    assert_eq!(result, RunResult::Complete);
    assert_eq!(*output.borrow(), ["20000"]);
    assert!(yields.get() >= 5, "{}", yields.get());
}

/// Tests compile-time literal folding and runtime string repetition.
#[test]
fn should_fold_literal_expressions() {
    let source = FOLDED;
    let program = program_for_code(source);
    let ops = &program.code(program.entry()).ops;
    let binaries = ops
        .iter()
        .filter(|op| matches!(op, Op::Binary { .. } | Op::AssignLocalBinary { .. }))
        .count();

    assert_eq!(binaries, 1, "only `\"ab\" * 2` stays: {ops:?}");
}

#[test]
fn should_give_folded_literals_the_values_the_vm_computes() {
    let code = format!("{FOLDED}\nprint x\nprint y\nprint z\nprint w\nprint v\nprint 1 / 0");

    let output = printed(&code);

    assert_eq!(
        output,
        ["7", "\"ab1\"", "-5.5", "1", "\"abab\"", "Infinity"]
    );
}

/// Function bodies whose locals no code can see as a map keep them in slots; the others keep a map per call.
#[test]
fn should_give_slots_only_to_bodies_without_map_access() {
    let bodies = [
        "b = a + 1\nfor i in [1]\n  b = b + i\nend for\nreturn b",
        "return self.x + super.y",
        "return locals",
        "g = function\n  return a\nend function",
        "debugger",
        "outer = 1",
        "__isa = {}",
        "return a.locals + a.globals",
    ];
    let slots = bodies.map(has_slots);

    assert!(slots[0]);
    assert!(slots[1]);
    assert!(!slots[2]);
    assert!(!slots[3]);
    assert!(!slots[4]);
    assert!(!slots[5]);
    assert!(!slots[6]);
    assert!(slots[7]);
}

/// Slot reads feeding an operation, condition or assignment become its operands; the example loop runs as 7 ops.
#[test]
fn should_fold_slot_reads_into_the_ops_using_them() {
    let source = "f = function(n)\n  total = 0\n  a = 1\n  b = 2\n  i = 0\n  while i < n\n    total = (total + a + b) % 999983\n    a = b\n    b = total\n    i = i + 1\n  end while\n  return total\nend function";
    let program = program_for_code(source);
    let entry = program.code(program.entry());
    let function = entry.ops.iter().find_map(|op| match op {
        Op::Function { code, .. } => Some(*code),
        _ => None,
    });
    let code = program.code(function.expect("a function"));
    let (end, target) = code
        .ops
        .iter()
        .enumerate()
        .find_map(|(index, op)| match op {
            Op::SlotBinaryJumpIf { target, .. } => Some((index, *target as usize)),
            _ => None,
        })
        .expect("the loop condition");
    let body = &code.ops[target..=end];

    assert!(
        matches!(
            body,
            [
                Op::SlotBinary {
                    left: Operand::Slot(_),
                    right: Operand::Slot(_),
                    to: Destination::Stack,
                    ..
                },
                Op::SlotBinary {
                    left: Operand::Stack,
                    right: Operand::Slot(_),
                    ..
                },
                Op::SlotBinary {
                    left: Operand::Stack,
                    right: Operand::Value(_),
                    to: Destination::Slot(_),
                    ..
                },
                Op::AssignSlot {
                    value: Operand::Slot(_),
                    ..
                },
                Op::AssignSlot {
                    value: Operand::Slot(_),
                    ..
                },
                Op::SlotBinary {
                    left: Operand::Slot(_),
                    right: Operand::Value(_),
                    to: Destination::Slot(_),
                    ..
                },
                Op::SlotBinaryJumpIf {
                    left: Operand::Slot(_),
                    right: Operand::Slot(_),
                    ..
                },
            ]
        ),
        "{body:?}"
    );

    let last = code
        .ops
        .iter()
        .rposition(|op| matches!(op, Op::Return(Some(Value::Null))))
        .expect("the closing return");

    assert!(!code.fallbacks.is_empty());
    assert!(
        code.fallbacks
            .iter()
            .all(|&(_, start)| start as usize > last)
    );
}

/// Without slots, variable reads feeding an operation, condition or assignment become its operands; the example loop runs as 5 ops.
#[test]
fn should_fold_variable_reads_into_the_ops_using_them() {
    let source = "a = 0\nb = 1\ni = 0\nn = 10\nwhile i < n\n  c = a + b\n  a = b\n  b = c\n  i = i + 1\nend while";
    let program = program_for_code(source);
    let code = program.code(program.entry());
    let (end, target) = code
        .ops
        .iter()
        .enumerate()
        .find_map(|(index, op)| match op {
            Op::VariableBinaryJumpIf { target, .. } => Some((index, *target as usize)),
            _ => None,
        })
        .expect("the loop condition");
    let body = &code.ops[target..=end];

    assert!(
        matches!(
            body,
            [
                Op::VariableBinary {
                    left: VariableOperand::Variable(_),
                    right: VariableOperand::Variable(_),
                    to: VariableDestination::Local(_),
                    ..
                },
                Op::CopyVariable { .. },
                Op::CopyVariable { .. },
                Op::VariableBinary {
                    left: VariableOperand::Variable(_),
                    right: VariableOperand::Value(_),
                    to: VariableDestination::Local(_),
                    ..
                },
                Op::VariableBinaryJumpIf {
                    left: VariableOperand::Variable(_),
                    right: VariableOperand::Variable(_),
                    ..
                },
            ]
        ),
        "{body:?}"
    );

    let stop = code
        .ops
        .iter()
        .position(|op| matches!(op, Op::Jump(Code::END)))
        .expect("a jump past the end");

    assert_eq!(stop, end + 1);
    assert!(
        code.fallbacks
            .iter()
            .all(|&(_, start)| start as usize > stop)
    );
}

/// A loop or `if` ending the code jumps to its end, which ends the program before the ops waiting there.
#[test]
fn should_end_at_the_end_of_code_with_folded_variables() {
    let code = "i = 0\nwhile i < 3\n  i = i + 1\nend while\nif i > 2 then\n  print i\nend if";

    let printed = printed(code);

    assert_eq!(printed, ["3"]);
}

/// A folded variable holding a function still calls it, from operations, assignments and loop conditions, in read order.
#[test]
fn should_call_functions_read_from_folded_variables() {
    let code = r#"
        calls = []
        g = function()
          calls.push "g"
          return 2
        end function
        h = function()
          calls.push "h"
          return 1
        end function
        x = g + h
        y = g
        i = 0
        while i < g
          i = i + 1
        end while
        print [x, y, i]
        print calls.len
        print calls[:2]
    "#;

    let printed = printed(code);

    assert_eq!(printed, ["[3, 2, 2]", "6", "[\"g\", \"h\"]"]);
}

/// A folded variable in no scope raises the error of the read it was folded from, at its line.
#[test]
fn should_report_folded_variables_in_no_scope_at_their_read() {
    let code = "a = 1\nb = a + 1\nc = b\nd = a +\n  missing\nprint d";
    let (printed, error) = run_code(code);

    assert!(printed.is_empty());

    let error = error.expect("missing is in no scope");

    assert_eq!(
        error.to_string(),
        "Undefined Identifier: 'missing' is unknown in this context"
    );
    assert_eq!(error.stack.first().map(|location| location.line), Some(5));
}

/// In a function without slots, a folded global read keeps reading it until an assignment makes a local of that name.
#[test]
fn should_read_the_local_a_folded_assignment_made() {
    let code = r#"
        x = 10
        f = function()
          locals.y = 0
          i = 0
          while i < 3
            x = x + 1
            i = i + 1
          end while
          z = x
          return [x, z, globals.x, i]
        end function
        print f
        print x
    "#;

    let printed = printed(code);

    assert_eq!(printed, ["[13, 13, 10, 3]", "10"]);
}

/// Tests loop state when the body mutates the locals map or calls another function.
#[test]
fn should_step_loops_through_changed_locals() {
    let code = r#"
        f = function(items)
          seen = []
          for x in items
            seen.push x
            if x == 2 then
              remove locals, "__x_idx"
              locals["y"] = 0
              __x_idx = 3
            end if
            if x == 5 then remove locals, "x"
          end for
          return seen
        end function
        print f([1, 2, 3, 4, 5, 6, 7])
        print f([9, 8])
    "#;

    let printed = printed(code);

    assert_eq!(printed, ["[1, 2, 5, 6, 7]", "[9, 8]"]);
}

/// Tests MiniScript 1-compatible loop index increments and final values.
#[test]
fn should_retain_the_loop_index_as_miniscript_does() {
    let code = r#"
        for k in [5, 6]
        end for
        print [k, __k_idx]
        f = function
          for k in [5, 6]
          end for
          for e in []
          end for
          for r in range(1, 3)
          end for
          return [k, __k_idx, __e_idx, __r_idx]
        end function
        print f
        for kv in {"a": 1, "b": 2}
        end for
        for b in [1, 2, 3]
          if b == 2 then break
        end for
        print [__kv_idx, __b_idx]
        __x_idx = 10
        g = function(items)
          seen = []
          for x in items
            seen.push x
            if x == 2 then remove locals, "__x_idx"
          end for
          return [seen, __x_idx]
        end function
        print g(range(1, 14))
    "#;

    let printed = printed(code);

    assert_eq!(
        printed,
        [
            "[6, 2]",
            "[6, 2, 0, 3]",
            "[2, 1]",
            "[[1, 2, 12, 13, 14], 14]"
        ]
    );
}

/// A loop whose body removes its index from every scope fails at the loop, as reading any missing variable does.
#[test]
fn should_fail_a_loop_whose_index_is_in_no_scope() {
    let code = "f = function(items)\n  for x in items\n    if x == 2 then remove locals, \"__x_idx\"\n  end for\nend function\nf [1, 2, 3]";
    let (_, error) = run_code(code);
    let error = error.expect("the index is in no scope");

    assert_eq!(
        error.to_string(),
        "Undefined Identifier: '__x_idx' is unknown in this context"
    );
    assert_eq!(error.stack.first().map(|location| location.line), Some(2));
}

/// Tests comparison jumps that do not materialize their result.
#[test]
fn should_branch_on_comparisons_as_their_results_would() {
    let code = r#"
        check = function(a, b)
          r = []
          if a < b then r.push "lt" else r.push "ge"
          if a == b then r.push "eq"
          if a != b then r.push "ne"
          if a <= 2 then r.push "le2"
          if 2 > a then r.push "2gt"
          if a > b and b >= 0 then r.push "gt-and"
          if a > b or b < -5 then r.push "or"
          if a % 2 then r.push "odd"
          if a - b then r.push "differ"
          if a * 1 < b * 1 then r.push "stack"
          if "" + a < "2" then r.push "str"
          if a < null then r.push "null"
          n = 0
          while a < b and n < 10
            a = a + 1
            n = n + 1
          end while
          r.push n
          return r
        end function
        pairs = [[1, 3], [3, 1], [2, 2], [0/0, 1], [-1/0, -10], [1.5, 1], [-0, 0]]
        for p in pairs
          print check(p[0], p[1])
        end for
        for p in pairs
          a = p[0]
          b = p[1]
          r = []
          if a < b then r.push "lt" else r.push "ge"
          if a == b then r.push "eq"
          if a != b then r.push "ne"
          if a <= 2 then r.push "le2"
          if a > b and b >= 0 then r.push "gt-and"
          if a > b or b < -5 then r.push "or"
          if a % 2 then r.push "odd"
          if a - b then r.push "differ"
          if a < null then r.push "null"
          n = 0
          while a < b and n < 10
            a = a + 1
            n = n + 1
          end while
          r.push n
          print r
        end for
    "#;

    let printed = printed(code);

    assert_eq!(
        printed,
        [
            r#"["lt", "ne", "le2", "2gt", "odd", "differ", "stack", "str", 2]"#,
            r#"["ge", "ne", "gt-and", "or", "odd", "differ", 0]"#,
            r#"["ge", "eq", "le2", 0]"#,
            r#"["ge", "ne", "odd", "differ", 0]"#,
            r#"["lt", "ne", "le2", "2gt", "or", "odd", "differ", "stack", "str", "null", 10]"#,
            r#"["ge", "ne", "le2", "2gt", "gt-and", "or", "odd", "differ", "str", 0]"#,
            r#"["ge", "eq", "le2", "2gt", "str", 0]"#,
            r#"["lt", "ne", "le2", "odd", "differ", 2]"#,
            r#"["ge", "ne", "gt-and", "or", "odd", "differ", 0]"#,
            r#"["ge", "eq", "le2", 0]"#,
            r#"["ge", "ne", "odd", "differ", 0]"#,
            r#"["lt", "ne", "le2", "or", "odd", "differ", "null", 10]"#,
            r#"["ge", "ne", "le2", "gt-and", "or", "odd", "differ", 0]"#,
            r#"["ge", "eq", "le2", 0]"#,
        ]
    );
}

/// Members are found along the `__isa` chain wherever a map holds its `__isa` entry, by name and by any key.
#[test]
fn should_find_members_along_the_isa_chain() {
    let code = r#"
        Base = {"greet": "hi"}
        Mid = {"a": 1, "__isa": Base}
        obj = {"b": 2, "c": 3, "__isa": Mid}
        print obj.greet
        print obj["greet"]
        print obj.a + obj["a"]
        print obj isa Base
        k = "gr" + "eet"
        print obj[k]
        loop = {}
        loop.__isa = loop
        loop.x = 5
        print loop.x
    "#;

    let printed = printed(code);

    assert_eq!(printed, [r#""hi""#, r#""hi""#, "2", "1", r#""hi""#, "5"]);
}

/// Grey Hack's fork of MiniScript reads elements of null, numbers and functions as null and takes keys of the wrong
/// type as index 0, where MiniScript 1.6.2 fails; both word their errors as their `MiniscriptException`s do.
#[test]
fn should_index_as_each_dialect_does() {
    let cases = [
        (
            "x = null\nprint x[0]",
            "null",
            "Null Reference Exception: can't index into null",
        ),
        (
            "x = 5\nprint x[0]",
            "null",
            "Type Exception: can't index into this type",
        ),
        (
            "print [1][5]",
            "Index Error: list index (5) out of range (-1 to 0)",
            "Index Error (list index 5 out of range)",
        ),
        (
            "print \"abc\"[-4]",
            "Index Error: string index (-4) out of range (-3 to 2)",
            "Index Error (string index -4 out of range)",
        ),
        (
            "print [1][null]",
            "list item is null",
            "List index must be numeric",
        ),
        (
            "print \"abc\"[null]",
            "Index Error (index is null)",
            "String index must be numeric",
        ),
        ("print [7][[1]]", "7", "List index must be numeric"),
        (
            "x = {}\nprint x[null]",
            "Invalid map key null",
            "Key Not Found: 'null' not found in map",
        ),
        (
            "f = function\n  return self.n\nend function\nprint f",
            "null",
            "Undefined Identifier: 'self' is unknown in this context",
        ),
    ];

    for (code, grey_hack, mini_script) in cases {
        let outcome = |dialect| match run_code_as(code, dialect) {
            (_, Some(error)) => error.to_string(),
            (printed, None) => printed.join("\n"),
        };

        assert_eq!(outcome(Dialect::Greybel), grey_hack, "{code}");
        assert_eq!(outcome(Dialect::MiniScript), mini_script, "{code}");
    }
}

/// Both dialects word the errors they share as MiniScript does.
#[test]
fn should_word_errors_as_mini_script_does() {
    let cases = [
        (
            "x = {\"a\": 1}\nprint x.b",
            "Key Not Found: 'b' not found in map",
        ),
        ("print \"abc\".foo", "Key Not Found: 'foo' not found in map"),
        (
            "x = null\nprint x.n",
            "Type Error (while attempting to look up n)",
        ),
        ("x = null\nx.a = 1", "can't set indexed element of null"),
        (
            "x = \"abc\"\nx[0] = \"d\"",
            "can't set an indexed element in this type",
        ),
        (
            "x = [1, 2]\nx[-3] = 0",
            "Index Error (list index -3 out of range)",
        ),
        ("x = [1]\nx[null] = 0", "Index Error (list index null)"),
        (
            "print \"a\" * \"b\"",
            "got a Miniscript.ValString where a Miniscript.ValNumber was required (string replication)",
        ),
        (
            "print [1] + null",
            "got null where a Miniscript.ValList was required (list concatenation)",
        ),
        (
            "print {} + 1",
            "got a Miniscript.ValNumber where a Miniscript.ValMap was required (map combination)",
        ),
        (
            "print [1] / \"x\"",
            "got a Miniscript.ValString where a Miniscript.ValNumber was required (list division)",
        ),
    ];

    for (code, expected) in cases {
        for dialect in [Dialect::Greybel, Dialect::MiniScript] {
            let (_, error) = run_code_as(code, dialect);

            assert_eq!(error.expect(code).to_string(), expected, "{code}");
        }
    }
}

/// `"a" - 1` and `-"a"` are null, a list assigned at a string key sets element 0, as in MiniScript.
#[test]
fn should_operate_on_mixed_types_as_mini_script_does() {
    let code = "print \"abc\" - 1\nprint -\"a\"\nprint -[1]\nprint -null\nx = [5, 6]\nx[\"1\"] = 1\nprint x";

    let printed = printed(code);

    assert_eq!(printed, ["null", "null", "null", "0", "[1, 6]"]);
}

/// MiniScript 1.6.2 fails a lookup beyond 256 `__isa` steps.
#[test]
fn should_fail_past_the_isa_depth_in_miniscript() {
    let chain = |n: usize| {
        format!(
            "m = []\nfor i in range(0, {last})\n  m.push {{}}\nend for\n\
             for i in range(0, {parent})\n  m[i].__isa = m[i + 1]\nend for\n",
            last = n - 1,
            parent = n - 2,
        )
    };
    let found = format!(
        "{}m[257].named = 1\nm[257][5] = 2\nprint m[0].named + m[0][5]\nprint m[0] isa m[258]",
        chain(300)
    );
    let too_deep = [
        format!("{}m[258].named = 1\nprint m[0].named", chain(300)),
        format!("{}print m[0] isa m[259]", chain(300)),
        format!("{}print m[0].missing", chain(258)),
        "loop = {}\nloop.__isa = loop\nprint loop[1]".to_owned(),
        "loop = {}\nloop.__isa = loop\nprint loop isa {}".to_owned(),
    ];
    let (printed, failure) = run_code_as(&found, Dialect::MiniScript);
    let error = |code: &str| {
        let (_, failure) = run_code_as(code, Dialect::MiniScript);

        failure.expect("fails").to_string()
    };
    let errors = too_deep.map(|code| error(&code));
    let missing = error(&format!("{}print m[0][7]", chain(258)));

    assert_eq!(failure, None);
    assert_eq!(printed, ["3", "1"]);
    for message in errors {
        assert_eq!(message, "__isa depth exceeded (perhaps a reference loop?)");
    }

    assert_eq!(missing, "Key Not Found: '7' not found in map");
}

/// Grey Hack's `ValSeqElem.Resolve` searches 1002 maps of an `__isa` chain and then reads null, so a loop reads as null.
#[test]
fn should_read_null_past_the_isa_depth_in_grey_hack() {
    let chain = "m = []\nfor i in range(0, 1099)\n  m.push {}\nend for\nfor i in range(0, 1098)\n  m[i].__isa = m[i + 1]\nend for\n";
    let code = format!(
        "{chain}m[1001].named = 1\nprint m[0].named\nm[1002].far = 1\nprint m[0].far\nloop = {{}}\nloop.__isa = loop\nprint loop.missing"
    );

    let printed = printed(&code);

    assert_eq!(printed, ["1", "null", "null"]);
}

/// A folded slot holding a function still calls it, from operations, assignments and loop conditions.
#[test]
fn should_call_functions_read_from_folded_slots() {
    let code = r#"
        two = function()
          return 2
        end function
        f = function(g)
          x = g + 1
          y = g
          i = 0
          while i < g
            i = i + 1
          end while
          return [x, y, i]
        end function
        print f(@two)
    "#;

    let printed = printed(code);

    assert_eq!(printed, ["[3, 2, 2]"]);
}

/// A folded slot not assigned yet reads its name through the scopes, and an error doing so points at the read.
#[test]
fn should_read_unset_folded_slots_through_the_scopes() {
    let code = "x = 10\nf = function()\n  y = x + 1\n  x = 5\n  return y + x\nend function\nprint f\ng = function()\n  y = z + 1\n  z = 1\nend function\ng";
    let (printed, error) = run_code(code);

    assert_eq!(printed, ["16"]);

    let error = error.expect("z is in no scope");

    assert_eq!(error.stack.first().map(|location| location.line), Some(9));
}

#[test]
fn should_treat_slot_locals_like_a_locals_map() {
    let code = r#"
        x = "global"
        f = function(a, b = 2)
          print x
          x = "local"
          print x
          total = 0
          for item in [a, b]
            total = total + item
          end for
          print total
          print item
          print __item_idx
          print @g
          return [a, b]
        end function
        g = 7
        print f(1)
        print x
        Base = {"name": "base"}
        Base.hello = function
          return "hello " + self.name
        end function
        Child = new Base
        Child.name = "child"
        Child.hello = function
          return super.hello + "!"
        end function
        print (new Child).hello
        h = function(self)
          return self.name
        end function
        print h(Child)
        "#;

    let output = printed(code);

    assert_eq!(
        output,
        [
            "\"global\"",
            "\"local\"",
            "3",
            "2",
            "2",
            "7",
            "[1, 2]",
            "\"global\"",
            "\"hello child!\"",
            "\"child\""
        ]
    );
}

/// Injecting while a body with slot locals runs reads and assigns them, and names the injected code adds stay visible to the body.
#[test]
fn should_read_and_assign_slot_locals_from_injected_code() {
    let (interpreter, output) = interpreter_for_code(
        "f = function(x)\n  y = x * 2\n  print y\n  print z\nend function\nz = 0\nf(5)",
    );
    let mut interpreter = interpreter.with_scheduler(OneAtATime);

    block_on(interpreter.start()).expect("starts");
    enter_call(&mut interpreter);
    block_on(interpreter.inject("print x\nx = 40\nz = 9")).expect("injects");
    resume_to_end(&mut interpreter).expect("runs");
    assert_eq!(*output.borrow(), ["5", "80", "9"]);
}

/// Reading `self` never calls it, whether the body keeps its locals in slots or in a map.
#[test]
fn should_not_call_a_function_held_by_self() {
    let code = r#"
        funcRef.mapBody = function
          x = locals
          return self
        end function
        funcRef.slotBody = function
          return self
        end function
        h = function
          return 5
        end function
        print (@h).mapBody
        print (@h).slotBody
        "#;

    let output = printed(code);

    assert_eq!(output, ["FUNCTION()", "FUNCTION()"]);
}

/// Tests that removals and shadowing invalidate cached variable locations.
#[test]
fn should_read_cached_variables_correctly() {
    let code = r#"
        x = 1
        y = 2
        for i in [0, 1]
          print y
          if i == 0 then remove(globals, "x")
        end for
        x = "global"
        f = function
          g = function
          end function
          for i in [0, 1]
            print x
            x = "local"
          end for
        end function
        f
        "#;

    let output = printed(code);

    assert_eq!(output, ["2", "2", "\"global\"", "\"local\""]);
}

/// Numeric literals parse as C# MiniScript does (`double.TryParse` on Grey Hack's runtime), with its rounding and out-of-range rejection.
#[test]
fn should_round_numeric_literals_as_the_reference() {
    let code = "print 565162807e-282 == 5.651628070000001e-274";

    let printed = printed(code);

    assert_eq!(printed, ["1"]);
}

#[test]
fn should_reject_out_of_range_numeric_literals() {
    let (mut interpreter, _) = interpreter_for_code("x = 1e400");
    let result = run_to_end(&mut interpreter);
    let error = result.expect_err("rejects the literal");

    assert!(
        error.to_string().contains("invalid numeric literal: 1e400"),
        "{error}"
    );
}

#[test]
fn should_nest_calls_until_the_program_runs_out_of_memory() {
    let source = "f = function\n  f\nend function\nf";
    let options = InterpreterOptions {
        memory_limit: 1 << 22,
        ..constant_clock_options()
    };
    let (mut interpreter, _) = interpreter_with(source, options);

    let error = runtime_error(run_to_end(&mut interpreter).expect_err("runs out of memory"));

    assert_eq!(
        error.to_string(),
        "Max memory usage reached. Script execution interrupted"
    );
}

#[test]
fn should_nest_calls_without_a_depth_limit() {
    let source = "f = function(n)\n  if n == 0 then return 0\n  return f(n - 1) + 1\nend function\nprint f(50000)";

    let (printed, error) = run_code(source);

    assert_eq!(error, None);
    assert_eq!(printed, ["50000"]);
}

/// MiniScript looks a name up in the literal `null` as null, where a variable holding null fails the lookup.
#[test]
fn should_read_null_members_of_the_literal_null() {
    let source = "print null.n\nprint (null).n\nprint @null.n\nx = null.len\nprint x";

    for dialect in [Dialect::GreyScript, Dialect::MiniScript] {
        let (printed, error) = run_code_as(source, dialect);

        assert_eq!(error, None, "{dialect:?}");
        assert_eq!(printed, ["null", "null", "null", "null"], "{dialect:?}");
    }

    let (_, error) = run_code_as("x = null\nprint x.n", Dialect::GreyScript);

    assert_eq!(
        error.map(|error| error.to_string()).as_deref(),
        Some("Type Error (while attempting to look up n)")
    );
}

#[test]
fn should_follow_each_dialect_with_new_isa_null_and_parenthesized_references() {
    let source = "print 1 isa null\nprint null isa null\nf = function\n  return 5\nend function\nm = {\"f\": @f}\nprint (@m.f)\nprint typeof(new 42)";
    let (printed, error) = run_code_as(source, Dialect::GreyScript);

    assert_eq!(error, None);
    assert_eq!(printed, ["1", "1", "5", "\"map\""]);

    let source = "print 1 isa null\nprint null isa null\nf = function\n  return 5\nend function\nm = {\"f\": @f}\nprint typeof(@m.f)\nprint typeof((@m.f))";
    let (printed, error) = run_code_as(source, Dialect::MiniScript);

    assert_eq!(error, None);
    assert_eq!(printed, ["0", "1", "\"function\"", "\"function\""]);

    for (source, message) in [
        ("x = new 42", "argument to 'new' must be a map"),
        (
            "x = new string",
            "invalid use of 'new'; to create a string, use quotes, e.g. \"foo\"",
        ),
        (
            "x = new funcRef",
            "invalid use of 'new'; to create a function, use the 'function' keyword",
        ),
    ] {
        let (_, error) = run_code_as(source, Dialect::MiniScript);

        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some(message),
            "{source}"
        );
    }
}

#[test]
fn should_evaluate_a_shorthand_target_once_in_miniscript() {
    let source = "m = {\"v\": 1}\nf = function\n  print \"F\"\n  return m\nend function\nf.v += 2\nl = [1, 2]\ni = function\n  print \"I\"\n  return 1\nend function\nl[i] *= 10\nprint m.v\nprint l";

    let (printed, error) = run_code_as(source, Dialect::MiniScript);

    assert_eq!(error, None);
    assert_eq!(printed, ["\"F\"", "\"I\"", "3", "[1, 20]"]);

    let (printed, error) = run_code_as(source, Dialect::Greybel);

    assert_eq!(error, None);
    assert_eq!(
        printed,
        ["\"F\"", "\"F\"", "\"I\"", "\"I\"", "3", "[1, 20]"]
    );
}

#[test]
fn should_replicate_as_each_dialect_does() {
    let source = "print \"a\" * (1/0)\nprint [\"a\"] * (0/0)\nprint \"ab\" / 0\nprint \"ab\" * 2.5\nprint [1, 2] * 1.5";
    let (printed, error) = run_code_as(source, Dialect::GreyScript);

    assert_eq!(error, None);
    assert_eq!(printed, ["\"\"", "[]", "\"\"", "\"ababa\"", "[1, 2, 1]"]);

    let (printed, error) = run_code_as(source, Dialect::MiniScript);

    assert_eq!(error, None);
    assert_eq!(printed, ["null", "null", "null", "\"ababa\"", "[1, 2, 1]"]);

    for (source, message) in [
        ("x = [\"a\"] * 1e10", "list too large"),
        ("x = \"ab\" * 9000000", "string too large"),
        (
            "x = [\"a\"] * (1/0)",
            "Oops, a game error has occurred that has caused a crash in the script you just ran.\n\nPlease report the error along with the steps you have taken for review.\nThank you!",
        ),
    ] {
        let (_, error) = run_code_as(source, Dialect::GreyScript);

        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some(message),
            "{source}"
        );
    }
}
