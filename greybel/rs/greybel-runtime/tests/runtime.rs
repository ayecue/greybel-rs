//! Tests running and debugging through the editor-facing runtime protocol.

use std::cell::Cell;
use std::fmt::Write;
use std::rc::Rc;
use std::task::Poll;

use duplex_core::Error;
use greybel_runtime::Yielder;

use crate::support::editor::{
    Continue, Environment, Evaluate, EvaluationFailure, Exit, InspectionFailure, Launch,
    LaunchFailure, Pause, Scopes, SetVariable, StackTrace, StepMode, StopReason, Terminate,
    Variables,
};
use crate::support::runtime::{
    LOOP, Launched, PROGRAM, breakpoint, conditional, log, set_breakpoints, set_source_breakpoints,
    stop, values_at_stops,
};
use crate::support::{Harness, LIBRARY, MAIN, mock_launch, named};

#[test]
fn should_run_a_program_to_its_end() {
    let mut harness = Harness::new(&[(MAIN, PROGRAM)]);

    let launch = harness.launch(false, false);
    let exit = harness.wait(launch);

    assert_eq!(exit, Ok(Exit::Completion));
    // The program's prints reach the terminal in one batch, as the game sends them.
    assert_eq!(*harness.terminal.print_log.borrow(), ["2\n3"]);
}

#[test]
fn should_run_a_program_larger_than_the_default_message_limit() {
    let filler = (0..8_000).fold(String::new(), |mut filler, index| {
        let _ = writeln!(filler, "x{index} = {index}");

        filler
    });
    let program = format!("{filler}print \"done\"");

    assert!(program.len() > 64 << 10);

    let mut harness = Harness::new(&[(MAIN, program.as_str())]);

    let launch = harness.launch(false, false);
    let exit = harness.wait(launch);

    assert_eq!(exit, Ok(Exit::Completion));
    assert_eq!(*harness.terminal.print_log.borrow(), ["done"]);
}

#[test]
fn should_report_a_runtime_error_with_its_stack() {
    let mut harness = Harness::new(&[(
        MAIN,
        "f = function\n  return 1 / undefined_name\nend function\nf",
    )]);

    let launch = harness.launch(false, false);
    let exit = harness.wait(launch).expect("launched");

    let Exit::Crash { message, stack } = exit else {
        panic!("expected a failure, got {exit:?}");
    };

    assert!(message.contains("undefined_name"), "{message}");

    let lines: Vec<u32> = stack.iter().map(|location| location.line).collect();

    assert_eq!(lines, [2, 4]);
}

#[test]
fn should_refuse_a_program_with_a_syntax_error() {
    let mut harness = Harness::new(&[(MAIN, "if true then\nprint(1")]);

    let launch = harness.launch(false, false);
    let exit = harness.wait(launch);

    let Err(Error::Failure(LaunchFailure::Invalid {
        location: Some(location),
        ..
    })) = exit
    else {
        panic!("expected a syntax error with its place, got {exit:?}");
    };

    assert_eq!((location.uri.as_str(), location.line), (MAIN, 2));
}

#[cfg(not(feature = "game"))]
#[test]
fn should_refuse_to_run_in_the_game_here() {
    let mut harness = Harness::new(&[(MAIN, PROGRAM)]);

    let launch = harness.launch_in_game(7777, false);
    let exit = harness.wait(launch);

    assert_eq!(exit, Err(Error::Failure(LaunchFailure::Unavailable)));
}

#[test]
fn should_stop_at_a_breakpoint_and_continue() {
    let mut harness = Harness::new(&[(MAIN, PROGRAM)]);

    set_breakpoints(&mut harness, MAIN, &[7]);

    let launch = harness.launch(true, false);
    let mut launch = Box::pin(launch);

    harness.wait_for_stops(1);

    let printed_at_stop = harness.terminal.print_log.borrow().clone();

    harness.editor.send(&Continue {}).expect("sent");

    let exit = harness.wait(launch.as_mut());

    assert_eq!(harness.last_stop(), stop(StopReason::Breakpoint, 7));
    assert!(printed_at_stop.is_empty());
    assert_eq!(exit, Ok(Exit::Completion));
    // The program's prints reach the terminal in one batch, as the game sends them.
    assert_eq!(*harness.terminal.print_log.borrow(), ["2\n3"]);
}

#[test]
fn should_ignore_breakpoints_without_debugging() {
    let mut harness = Harness::new(&[(MAIN, PROGRAM)]);

    set_breakpoints(&mut harness, MAIN, &[7]);

    let launch = harness.launch(false, false);
    let exit = harness.wait(launch);

    assert_eq!(exit, Ok(Exit::Completion));
    assert!(harness.stops.borrow().is_empty());
}

#[test]
fn should_stop_on_entry() {
    let mut harness = Harness::new(&[(MAIN, PROGRAM)]);
    let _launch = Box::pin(harness.launch(true, true));

    harness.wait_for_stops(1);

    assert_eq!(harness.last_stop(), stop(StopReason::Entry, 1));
}

#[test]
fn should_step_over_into_and_out_of_calls() {
    let mut harness = Harness::new(&[(MAIN, PROGRAM)]);

    set_breakpoints(&mut harness, MAIN, &[5]);

    let _launch = Box::pin(harness.launch(true, false));

    harness.wait_for_stops(1);

    harness.step(StepMode::Over);
    harness.wait_for_stops(2);
    harness.step(StepMode::Into);
    harness.wait_for_stops(3);
    harness.step(StepMode::Out);
    harness.wait_for_stops(4);

    assert_eq!(
        *harness.stops.borrow(),
        [
            stop(StopReason::Breakpoint, 5),
            stop(StopReason::Step, 6),
            stop(StopReason::Step, 2),
            stop(StopReason::Step, 7),
        ]
    );
}

#[test]
fn should_show_the_stack_and_its_variables() {
    let mut harness = Harness::new(&[(MAIN, PROGRAM)]);

    set_breakpoints(&mut harness, MAIN, &[3]);

    let _launch = Box::pin(harness.launch(true, false));

    harness.wait_for_stops(1);

    let frames = harness.request(&StackTrace {}).expect("a stack");
    let scopes = harness
        .request(&Scopes {
            frame: frames[0].id,
        })
        .expect("scopes");
    let locals = harness
        .request(&Variables {
            reference: scopes[0].reference,
        })
        .expect("locals");

    let named: Vec<(&str, Option<u32>)> = frames
        .iter()
        .map(|frame| {
            (
                frame.name.as_str(),
                frame.location.as_ref().map(|location| location.line),
            )
        })
        .collect();

    assert_eq!(
        named,
        [("function main.src:2", Some(3)), ("main.src", Some(6))]
    );

    let scopes: Vec<&str> = scopes.iter().map(|scope| scope.name.as_str()).collect();

    assert_eq!(scopes, ["Locals", "Globals"]);

    let locals: Vec<(&str, &str, &str)> = locals
        .iter()
        .map(|variable| {
            (
                variable.name.as_str(),
                variable.value.as_str(),
                variable.type_name.as_str(),
            )
        })
        .collect();

    assert_eq!(locals, [("x", "1", "number"), ("y", "2", "number")]);
}

#[test]
fn should_expand_lists_and_maps() {
    let mut harness = Harness::new(&[(
        MAIN,
        "data = {\"name\": \"neo\", \"items\": [1, 2]}\nprint(data.name)",
    )]);

    set_breakpoints(&mut harness, MAIN, &[2]);

    let _launch = Box::pin(harness.launch(true, false));

    harness.wait_for_stops(1);

    let globals = harness.globals(0);
    let data = named(&globals, "data");

    let entries = harness
        .request(&Variables {
            reference: data.reference,
        })
        .expect("entries");
    let items = harness
        .request(&Variables {
            reference: named(&entries, "items").reference,
        })
        .expect("items");

    assert_eq!(data.type_name, "map");
    assert_ne!(data.reference, 0);

    let entries: Vec<(&str, &str)> = entries
        .iter()
        .map(|variable| (variable.name.as_str(), variable.value.as_str()))
        .collect();

    assert_eq!(entries, [("name", "\"neo\""), ("items", "[1, 2]")]);

    let items: Vec<(&str, &str)> = items
        .iter()
        .map(|variable| (variable.name.as_str(), variable.value.as_str()))
        .collect();

    assert_eq!(items, [("0", "1"), ("1", "2")]);
}

#[test]
fn should_evaluate_expressions_and_statements_where_it_stopped() {
    let mut harness = Harness::new(&[(MAIN, PROGRAM)]);

    set_breakpoints(&mut harness, MAIN, &[3]);

    let mut launch = Box::pin(harness.launch(true, false));

    harness.wait_for_stops(1);

    let value = harness.request(&Evaluate {
        code: "y + 40".into(),
    });
    let assigned = harness.request(&Evaluate {
        code: "y = 10".into(),
    });
    let failed = harness.request(&Evaluate { code: "1 +".into() });

    harness.editor.send(&Continue {}).expect("sent");

    let exit = harness.wait(launch.as_mut());

    let value = value.expect("evaluated").expect("a value");

    assert_eq!(
        (
            value.value.as_str(),
            value.type_name.as_str(),
            value.reference
        ),
        ("42", "number", 0)
    );
    assert_eq!(assigned, Ok(None));
    assert!(
        matches!(
            failed,
            Err(Error::Failure(EvaluationFailure::CodeError { .. }))
        ),
        "{failed:?}"
    );
    assert_eq!(exit, Ok(Exit::Completion));
    assert_eq!(*harness.terminal.print_log.borrow(), ["10\n11"]);
}

#[test]
fn should_refuse_to_inspect_a_running_program() {
    let mut harness = Harness::new(&[(MAIN, "while true\nend while")]);
    let _launch = Box::pin(harness.launch(true, false));

    harness.run_for(20);

    let frames = harness.request(&StackTrace {});

    assert_eq!(
        frames,
        Err(Error::Failure(InspectionFailure::ActiveProgram))
    );
}

#[test]
fn should_pause_a_running_program() {
    let mut harness = Harness::new(&[(MAIN, "i = 0\nwhile true\n  i = i + 1\nend while")]);
    let _launch = Box::pin(harness.launch(true, false));

    harness.run_for(20);

    harness.editor.send(&Pause {}).expect("sent");
    harness.wait_for_stops(1);

    let stop = harness.last_stop();

    assert_eq!(stop.reason, StopReason::Pause);
    assert!(matches!(stop.line, Some(2..=3)), "{stop:?}");
}

#[test]
fn should_pause_a_program_paused_before_it_started() {
    let mut harness = Harness::new(&[(MAIN, "i = 0\nwhile true\n  i = i + 1\nend while")]);
    let _launch = Box::pin(harness.launch(true, false));

    harness.editor.send(&Pause {}).expect("sent");
    harness.wait_for_stops(1);

    assert_eq!(harness.last_stop(), stop(StopReason::Pause, 1));
}

#[test]
fn should_ignore_a_pause_while_no_program_runs() {
    let mut harness = Harness::new(&[(MAIN, PROGRAM)]);

    harness.editor.send(&Pause {}).expect("sent");
    harness.run_for(20);

    let launch = harness.launch(true, false);
    let exit = harness.wait(launch);

    assert_eq!(exit, Ok(Exit::Completion));
    assert_eq!(harness.stop_lines(), []);
}

#[test]
fn should_wait_on_its_host_when_a_running_program_yields() {
    let yields = Rc::new(Cell::new(0));
    let counted = Rc::clone(&yields);
    let yielder: Yielder = Rc::new(move || {
        counted.set(counted.get() + 1);

        let mut waited = false;

        Box::pin(std::future::poll_fn(move |context| {
            if waited {
                return Poll::Ready(());
            }

            waited = true;
            context.waker().wake_by_ref();

            Poll::Pending
        }))
    });
    let mut harness = Harness::with_runtime(
        &[(MAIN, "i = 0\nwhile true\n  i = i + 1\nend while")],
        |runtime| runtime.with_yielder(yielder),
    );
    let _launch = Box::pin(harness.launch(true, false));

    harness.run_for(50);

    harness.editor.send(&Pause {}).expect("sent");
    harness.wait_for_stops(1);

    assert!(yields.get() > 0);
    assert_eq!(harness.last_stop().reason, StopReason::Pause);
}

#[test]
fn should_terminate_a_running_program() {
    let mut harness = Harness::new(&[(MAIN, "while true\nend while")]);
    let mut launch = Box::pin(harness.launch(false, false));

    harness.run_for(20);

    harness.editor.send(&Terminate {}).expect("sent");

    let exit = harness.wait(launch.as_mut());

    assert_eq!(exit, Ok(Exit::Termination));
}

#[test]
fn should_terminate_a_stopped_program() {
    let mut harness = Harness::new(&[(MAIN, PROGRAM)]);
    let mut launch = Box::pin(harness.launch(true, true));

    harness.wait_for_stops(1);

    harness.editor.send(&Terminate {}).expect("sent");

    let exit = harness.wait(launch.as_mut());

    assert_eq!(exit, Ok(Exit::Termination));
    assert!(harness.terminal.print_log.borrow().is_empty());
}

#[test]
fn should_stop_in_a_file_the_program_imports() {
    let main = "import_code(\"/lib.src\")\nprint(double(4))";
    let library = "double = function(n)\n  return n * 2\nend function";
    let mut harness = Harness::new(&[(MAIN, main), (LIBRARY, library)]);

    set_breakpoints(&mut harness, LIBRARY, &[2]);

    let mut launch = Box::pin(harness.launch(true, false));

    harness.wait_for_stops(1);

    let frames = harness.request(&StackTrace {}).expect("a stack");

    harness.editor.send(&Continue {}).expect("sent");

    let exit = harness.wait(launch.as_mut());

    let location = frames[0].location.as_ref().expect("a location");

    assert_eq!((location.uri.as_str(), location.line), (LIBRARY, 2));
    assert_eq!(exit, Ok(Exit::Completion));
    assert_eq!(*harness.terminal.print_log.borrow(), ["8"]);
}

#[test]
fn should_resolve_an_import_with_the_extensions_of_the_launch() {
    let library = "file:///project/lib.gs";
    let files = [
        (MAIN, "import_code(\"/lib\")\nprint(x)"),
        (library, "x = 1"),
    ];
    let mut resolving = Harness::new(&files);
    let mut not_resolving = Harness::new(&files);

    let resolved = resolving.send_launch(&mock_launch(false, false));
    let resolved = resolving.wait(resolved);
    let unresolved = not_resolving.send_launch(&Launch {
        extensions: vec!["src".to_owned()].into(),
        ..mock_launch(false, false)
    });
    let unresolved = not_resolving.wait(unresolved);

    assert_eq!(resolved, Ok(Exit::Completion));
    assert_eq!(*resolving.terminal.print_log.borrow(), ["1"]);
    assert!(
        matches!(
            unresolved,
            Err(Error::Failure(LaunchFailure::Invalid { .. }))
        ),
        "{unresolved:?}"
    );
}

#[test]
fn should_parse_in_strict_mode_when_the_launch_asks_for_it() {
    let files = [(MAIN, "x = 1\nx += 1\nprint(x)")];
    let mut tolerant = Harness::new(&files);
    let mut strict = Harness::new(&files);

    let tolerated = tolerant.send_launch(&mock_launch(false, false));
    let tolerated = tolerant.wait(tolerated);
    let rejected = strict.send_launch(&Launch {
        strict_mode: true,
        ..mock_launch(false, false)
    });
    let rejected = strict.wait(rejected);

    assert_eq!(tolerated, Ok(Exit::Completion));
    assert!(
        matches!(rejected, Err(Error::Failure(LaunchFailure::Invalid { .. }))),
        "{rejected:?}"
    );
}

#[test]
fn should_generate_the_mock_world_from_the_launchs_seed() {
    let program = "print(get_router.bssid_name)";
    let bssids = ["first", "first", "second"].map(|seed| {
        let mut harness = Harness::new(&[(MAIN, program)]);
        let launch = harness.send_launch(&Launch {
            environment: Environment::Mock {
                seed: seed.into(),
                memory_limit: 1024,
                user: None,
            },
            ..mock_launch(false, false)
        });

        assert_eq!(harness.wait(launch), Ok(Exit::Completion));

        harness.terminal.print_log.borrow().clone()
    });

    assert_eq!(bssids[0], bssids[1], "the same seed makes the same world");
    assert_ne!(bssids[0], bssids[2], "another seed makes another world");
}

#[test]
fn should_read_environment_variables_the_editor_gives() {
    let mut harness = Harness::new(&[(MAIN, "print(#envar NAME)")]);

    let launch = harness.launch_with(false, false, &[("NAME", "neo")]);
    let exit = harness.wait(launch);

    assert_eq!(exit, Ok(Exit::Completion));
    assert_eq!(*harness.terminal.print_log.borrow(), ["neo"]);
}

#[test]
fn should_read_the_programs_input_from_the_editor() {
    let mut harness = Harness::new(&[(
        MAIN,
        "name = user_input(\"name: \")\nprint(\"hello \" + name)",
    )]);

    harness
        .terminal
        .lines
        .borrow_mut()
        .push_back("neo".to_owned());

    let launch = harness.launch(false, false);
    let exit = harness.wait(launch);

    assert_eq!(exit, Ok(Exit::Completion));
    assert_eq!(
        harness
            .terminal
            .print_log
            .borrow()
            .last()
            .map(String::as_str),
        Some("hello neo")
    );
}

#[test]
fn should_stop_only_where_a_breakpoints_condition_is_true() {
    let mut harness = Harness::new(&[(MAIN, LOOP)]);

    set_source_breakpoints(
        &mut harness,
        MAIN,
        vec![conditional(3, Some("i % 2 == 0"), None, None)],
    );

    let mut launch: Launched = Box::pin(harness.launch(true, false));

    let values = values_at_stops(&mut harness, &mut launch, "i");

    assert_eq!(values, ["2", "4"]);
    assert_eq!(*harness.terminal.print_log.borrow(), ["15"]);
}

#[test]
fn should_stop_by_a_breakpoints_hit_count() {
    let mut harness = Harness::new(&[(MAIN, LOOP)]);

    set_source_breakpoints(
        &mut harness,
        MAIN,
        vec![conditional(3, None, Some(">= 4"), None)],
    );

    let mut launch: Launched = Box::pin(harness.launch(true, false));

    let values = values_at_stops(&mut harness, &mut launch, "i");

    assert_eq!(values, ["4", "5"]);
}

#[test]
fn should_refuse_a_hit_count_that_is_no_number() {
    let mut harness = Harness::new(&[(MAIN, LOOP)]);

    let set = set_source_breakpoints(
        &mut harness,
        MAIN,
        vec![conditional(3, None, Some("often"), None)],
    );

    assert!(!set[0].verified);
    assert!(
        set[0]
            .message
            .as_deref()
            .is_some_and(|message| message.contains("often")),
        "{set:?}"
    );
}

#[test]
fn should_log_a_logpoints_message_without_stopping() {
    let mut harness = Harness::new(&[(MAIN, LOOP)]);

    set_source_breakpoints(
        &mut harness,
        MAIN,
        vec![conditional(
            3,
            Some("i > 3"),
            None,
            Some("i is {i}, total {total}"),
        )],
    );

    let launch = harness.launch(true, false);
    let exit = harness.wait(launch);

    assert_eq!(exit, Ok(Exit::Completion));
    assert!(harness.stops.borrow().is_empty());
    assert_eq!(
        *harness.logs.borrow(),
        [log("i is 4, total 6", 3), log("i is 5, total 10", 3)]
    );
}

#[test]
fn should_finish_a_step_over_a_call_with_a_breakpoint_that_passes() {
    let mut harness = Harness::new(&[(MAIN, PROGRAM)]);

    set_source_breakpoints(
        &mut harness,
        MAIN,
        vec![breakpoint(6), conditional(2, Some("x > 100"), None, None)],
    );

    let _launch = Box::pin(harness.launch(true, false));

    harness.wait_for_stops(1);

    harness.step(StepMode::Over);
    harness.wait_for_stops(2);

    assert_eq!(
        *harness.stops.borrow(),
        [stop(StopReason::Breakpoint, 6), stop(StopReason::Step, 7)]
    );
}

#[test]
fn should_set_a_variable_and_a_maps_entry() {
    let mut harness = Harness::new(&[(
        MAIN,
        "data = {\"name\": \"neo\"}\nx = 1\nprint(x)\nprint(data.name)",
    )]);

    set_breakpoints(&mut harness, MAIN, &[3]);

    let mut launch = Box::pin(harness.launch(true, false));

    harness.wait_for_stops(1);

    let scopes = harness.request(&Scopes { frame: 0 }).expect("scopes");
    let globals = harness
        .request(&Variables {
            reference: scopes[1].reference,
        })
        .expect("globals");
    let data = named(&globals, "data");

    let x = harness
        .request(&SetVariable {
            reference: scopes[0].reference,
            name: "x".into(),
            value: "20 + 1".into(),
        })
        .expect("set");
    let name = harness
        .request(&SetVariable {
            reference: data.reference,
            name: "name".into(),
            value: "\"trinity\"".into(),
        })
        .expect("set");

    harness.editor.send(&Continue {}).expect("sent");

    let exit = harness.wait(launch.as_mut());

    assert_eq!(
        (x.value.as_str(), name.value.as_str()),
        ("21", "\"trinity\"")
    );
    assert_eq!(data.evaluate_name.as_deref(), Some("globals[\"data\"]"));
    assert_eq!(exit, Ok(Exit::Completion));
    assert_eq!(*harness.terminal.print_log.borrow(), ["21\ntrinity"]);
}

#[test]
fn should_expand_an_evaluated_list() {
    let mut harness = Harness::new(&[(MAIN, PROGRAM)]);

    set_breakpoints(&mut harness, MAIN, &[7]);

    let _launch = Box::pin(harness.launch(true, false));

    harness.wait_for_stops(1);

    let list = harness
        .request(&Evaluate {
            code: "[a, b, [a]]".into(),
        })
        .expect("evaluated")
        .expect("a value");
    let items = harness
        .request(&Variables {
            reference: list.reference,
        })
        .expect("items");

    assert_eq!(
        (list.value.as_str(), list.type_name.as_str()),
        ("[1, 2, [1]]", "list")
    );

    let items: Vec<(&str, &str, Option<&str>)> = items
        .iter()
        .map(|item| {
            (
                item.name.as_str(),
                item.value.as_str(),
                item.evaluate_name.as_deref(),
            )
        })
        .collect();

    assert_eq!(
        items,
        [
            ("0", "1", Some("([a, b, [a]])[0]")),
            ("1", "2", Some("([a, b, [a]])[1]")),
            ("2", "[1]", Some("([a, b, [a]])[2]"))
        ]
    );
}

#[test]
fn should_name_the_files_of_a_program_that_launches_itself() {
    let main = "#include \"/inc.src\"\n#import m from \"/mod.src\"\nimport_code(\"/lib.src\")\nwhat = \"\"\nif params.len > 0 then what = params[0]\nif what == \"\" then\n  for path in [program_path, home_dir + \"/copy\"]\n    if path != program_path then get_shell.host_computer.File(program_path).copy(home_dir, \"copy\")\n    for name in [\"inc\", \"mod\", \"lib\", \"main\"]\n      print path + \" \" + name + \"=\" + get_shell.launch(path, name)\n    end for\n  end for\nend if\nif what == \"inc\" then crash_inc\nif what == \"mod\" then m.crash\nif what == \"lib\" then crash_lib\nif what == \"main\" then x = [1][8]";
    let mut harness = Harness::new(&[
        (MAIN, main),
        (
            "file:///project/inc.src",
            "crash_inc = function\n  return [1][5]\nend function",
        ),
        (
            "file:///project/mod.src",
            "module.exports = {}\nmodule.exports.crash = function\n  return [1][6]\nend function",
        ),
        (
            LIBRARY,
            "crash_lib = function\n\n  return [1][7]\nend function",
        ),
    ]);

    let launch = harness.launch(false, false);
    let exit = harness.wait(launch);

    assert_eq!(exit, Ok(Exit::Completion));

    // The program's own binary runs its code; a copy runs the built code, which its source map names.
    let crashes = [
        ("inc", 5, "/project/inc.src line 2"),
        ("mod", 6, "/project/mod.src line 3"),
        ("lib", 7, "/project/lib.src line 3"),
        ("main", 8, "/project/main.src line 17"),
    ];
    // One batch for each path: the copy sends the batch before it, as the game's `copy` does.
    let expected: Vec<String> = ["/home/test/myprogram", "/home/test/copy"]
        .iter()
        .map(|path| {
            crashes
                .iter()
                .flat_map(move |(name, index, at)| {
                    [
                        format!(
                            "<color=red>Runtime Error: Index Error: list index ({index}) out of range (-1 to 0) [{at}]</color>"
                        ),
                        format!("{path} {name}=1"),
                    ]
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect();

    assert_eq!(*harness.terminal.print_log.borrow(), expected);
}
