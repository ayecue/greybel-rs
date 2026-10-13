//! Tests running and debugging in headless Grey Hack through the editor-facing runtime protocol.

use greybel_test_support::game::RUN_TIME;

use crate::support::editor::{
    Continue, Evaluate, Exit, Scopes, SetVariable, SourceBreakpoint, StackTrace, StepMode,
    StopReason,
};
use crate::support::game::{in_game, set_breakpoints_on_io, variables_on_io};
use crate::support::runtime::{breakpoint, breakpoints_request, stop};
use crate::support::{Harness, LIBRARY, MAIN, named};

#[test]
fn should_run_a_program_in_the_game_and_show_what_it_prints() {
    in_game(env!("CARGO_TARGET_TMPDIR"), |port| async move {
        let mut harness = Harness::new(&[(
            MAIN,
            "print(\"hello \" + (1 + 1))\nprint(get_shell.host_computer.get_name.len > 0)",
        )]);

        let launch = harness.launch_in_game(port, false);
        let exit = harness.wait_on_io(launch, RUN_TIME).await;

        assert_eq!(exit, Ok(Exit::Completion));

        assert_eq!(harness.terminal.print_log.borrow().join("\n"), "hello 2\n1");
    });
}

#[test]
fn should_run_the_files_a_program_imports_from_the_editor() {
    in_game(env!("CARGO_TARGET_TMPDIR"), |port| async move {
        let main = "import_code(\"/lib.src\")\nprint(double(4))";
        let library = "double = function(n)\n  return n * 2\nend function";
        let mut harness = Harness::new(&[(MAIN, main), (LIBRARY, library)]);

        let launch = harness.launch_in_game(port, false);
        let exit = harness.wait_on_io(launch, RUN_TIME).await;

        assert_eq!(exit, Ok(Exit::Completion));
        assert_eq!(*harness.terminal.print_log.borrow(), ["8"]);
    });
}

#[test]
fn should_stop_at_a_breakpoint_in_the_game_step_and_continue() {
    in_game(env!("CARGO_TARGET_TMPDIR"), |port| async move {
        let mut harness = Harness::new(&[(MAIN, "a = 1\nb = a + 1\nprint(b)\nprint(a)")]);

        set_breakpoints_on_io(&mut harness, vec![breakpoint(2)]).await;

        let mut launch = Box::pin(harness.launch_in_game(port, true));

        harness.stops_on_io(1, RUN_TIME).await;

        let frames = harness
            .request_on_io(&StackTrace {})
            .await
            .expect("a stack");
        let scopes = harness
            .request_on_io(&Scopes { frame: 0 })
            .await
            .expect("scopes");
        let variables = variables_on_io(&mut harness, scopes[0].reference).await;

        harness.step(StepMode::Over);
        harness.stops_on_io(2, RUN_TIME).await;
        harness.editor.send(&Continue {}).expect("sent");

        let exit = harness.wait_on_io(launch.as_mut(), RUN_TIME).await;

        assert_eq!(
            *harness.stops.borrow(),
            [stop(StopReason::Breakpoint, 2), stop(StopReason::Step, 3)]
        );

        let location = frames[0].location.as_ref().expect("a location");

        assert_eq!((location.uri.as_str(), location.line), (MAIN, 2));
        assert!(
            variables
                .iter()
                .any(|variable| variable.name == "a" && variable.value == "1"),
            "{variables:?}"
        );
        assert_eq!(exit, Ok(Exit::Completion));
        assert_eq!(harness.terminal.print_log.borrow().join("\n"), "2\n1");
    });
}

#[test]
fn should_step_into_and_out_of_calls_in_the_game_and_show_every_frames_variables() {
    in_game(env!("CARGO_TARGET_TMPDIR"), |port| async move {
        let source = "f = function(x)\n  y = x * 2\n  return y\nend function\nnested = [[[[[1]]]]]\na = f(1)\nprint(a)";
        let mut harness = Harness::new(&[(MAIN, source)]);

        set_breakpoints_on_io(&mut harness, vec![breakpoint(6)]).await;

        let mut launch = Box::pin(harness.launch_in_game(port, true));

        harness.stops_on_io(1, RUN_TIME).await;

        harness.step(StepMode::Into);
        harness.stops_on_io(2, RUN_TIME).await;

        let frames = harness
            .request_on_io(&StackTrace {})
            .await
            .expect("a stack");
        let outer = harness
            .request_on_io(&Scopes { frame: 1 })
            .await
            .expect("scopes");
        let outer_globals = variables_on_io(&mut harness, outer[1].reference).await;
        let mut nested = named(&outer_globals, "nested").clone();
        let mut depth = 0;

        while nested.reference != 0 {
            nested = variables_on_io(&mut harness, nested.reference)
                .await
                .remove(0);
            depth += 1;
        }

        harness.step(StepMode::Out);
        harness.stops_on_io(3, RUN_TIME).await;
        harness.editor.send(&Continue {}).expect("sent");

        let exit = harness.wait_on_io(launch.as_mut(), RUN_TIME).await;

        assert_eq!(harness.stop_lines(), [Some(6), Some(2), Some(7)]);

        let names: Vec<&str> = frames.iter().map(|frame| frame.name.as_str()).collect();

        assert_eq!(names, ["function main.src:2", "main.src"]);
        assert_eq!((depth, nested.value.as_str()), (5, "1"));
        assert_eq!(exit, Ok(Exit::Completion));
        assert_eq!(*harness.terminal.print_log.borrow(), ["2"]);
    });
}

#[test]
fn should_evaluate_and_set_variables_in_the_game() {
    in_game(env!("CARGO_TARGET_TMPDIR"), |port| async move {
        let source = "data = {\"name\": \"neo\"}\nx = 1\nprint(x)\nprint(data.name)";
        let mut harness = Harness::new(&[(MAIN, source)]);

        set_breakpoints_on_io(&mut harness, vec![breakpoint(3)]).await;

        let mut launch = Box::pin(harness.launch_in_game(port, true));

        harness.stops_on_io(1, RUN_TIME).await;

        let scopes = harness
            .request_on_io(&Scopes { frame: 0 })
            .await
            .expect("scopes");
        let globals = variables_on_io(&mut harness, scopes[1].reference).await;
        let data = named(&globals, "data").clone();

        let list = harness
            .request_on_io(&Evaluate {
                code: "[x, x + 1]".into(),
            })
            .await
            .expect("evaluated")
            .expect("a value");
        let items = variables_on_io(&mut harness, list.reference).await;
        let invalid = harness
            .request_on_io(&Evaluate { code: "1 +".into() })
            .await;
        let x = harness
            .request_on_io(&SetVariable {
                reference: scopes[0].reference,
                name: "x".into(),
                value: "20 + 1".into(),
            })
            .await
            .expect("set");
        let name = harness
            .request_on_io(&SetVariable {
                reference: data.reference,
                name: "name".into(),
                value: "\"trinity\"".into(),
            })
            .await
            .expect("set");

        harness.editor.send(&Continue {}).expect("sent");

        let exit = harness.wait_on_io(launch.as_mut(), RUN_TIME).await;

        assert_eq!(
            (list.value.as_str(), list.type_name.as_str()),
            ("[1, 2]", "list")
        );

        let items: Vec<&str> = items.iter().map(|item| item.value.as_str()).collect();

        assert_eq!(items, ["1", "2"]);
        assert!(invalid.is_err(), "{invalid:?}");
        assert_eq!(
            (x.value.as_str(), name.value.as_str()),
            ("21", "\"trinity\"")
        );
        assert_eq!(harness.stops.borrow().len(), 1);
        assert_eq!(exit, Ok(Exit::Completion));
        assert_eq!(
            harness.terminal.print_log.borrow().join("\n"),
            "21\ntrinity"
        );
    });
}

#[test]
fn should_stop_by_conditions_hit_counts_and_log_in_the_game() {
    in_game(env!("CARGO_TARGET_TMPDIR"), |port| async move {
        let source = "total = 0\nfor i in range(1, 5)\n  total = total + i\n  wait(0.01)\nend for\nprint(total)";
        let mut harness = Harness::new(&[(MAIN, source)]);

        set_breakpoints_on_io(
            &mut harness,
            vec![
                SourceBreakpoint {
                    condition: Some("i % 2 == 0".to_owned()),
                    hit_condition: Some("2".to_owned()),
                    ..breakpoint(3)
                },
                SourceBreakpoint {
                    log_message: Some("i is {i}".to_owned()),
                    ..breakpoint(4)
                },
            ],
        )
        .await;

        let mut launch = Box::pin(harness.launch_in_game(port, true));

        harness.stops_on_io(1, RUN_TIME).await;

        let at_stop = harness
            .request_on_io(&Evaluate { code: "i".into() })
            .await
            .expect("evaluated")
            .expect("a value");

        set_breakpoints_on_io(&mut harness, vec![breakpoint(6)]).await;
        harness.editor.send(&Continue {}).expect("sent");
        harness.stops_on_io(2, RUN_TIME).await;
        harness.editor.send(&Continue {}).expect("sent");

        let exit = harness.wait_on_io(launch.as_mut(), RUN_TIME).await;

        assert_eq!(at_stop.value, "4");
        assert_eq!(harness.stop_lines(), [Some(3), Some(6)]);

        let logs: Vec<String> = harness
            .logs
            .borrow()
            .iter()
            .map(|log| log.text.clone())
            .collect();

        assert_eq!(logs, ["i is 1", "i is 2", "i is 3"]);
        assert_eq!(exit, Ok(Exit::Completion));
        assert_eq!(*harness.terminal.print_log.borrow(), ["15"]);
    });
}

#[test]
fn should_name_the_files_of_a_program_that_launches_itself_in_the_game() {
    in_game(env!("CARGO_TARGET_TMPDIR"), |port| async move {
        let main = "#include \"/inc.src\"\n#import m from \"/mod.src\"\nimport_code(\"/lib.src\")\nwhat = \"\"\nif params.len > 0 then what = params[0]\nif what == \"\" then\n  for name in [\"inc\", \"mod\", \"lib\", \"main\"]\n    print name + \"=\" + get_shell.launch(program_path, name)\n  end for\nend if\nif what == \"inc\" then crash_inc\nif what == \"mod\" then m.crash\nif what == \"lib\" then crash_lib\nif what == \"main\" then x = [1][8]";
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

        let launch = harness.launch_in_game(port, false);
        let exit = harness.wait_on_io(launch, RUN_TIME).await;

        assert_eq!(exit, Ok(Exit::Completion));

        // Each launch runs the program's binary, whose source map names the file and line of each error.
        let crash = |index: u32, at: &str, stack: &[&str]| {
            let mut lines = vec![format!(
                "<color=red>Runtime Error: Index Error: list index ({index}) out of range (-1 to 0) [{at}]</color>"
            )];

            lines.extend(
                stack
                    .iter()
                    .map(|frame| format!("<color=red>  {frame}</color>")),
            );

            lines.join("\n")
        };
        let expected = [
            crash(
                5,
                "/project/inc.src line 2",
                &["/project/inc.src:2", "/project/main.src:11"],
            ),
            "inc=1".to_owned(),
            crash(
                6,
                "/project/mod.src line 3",
                &["/project/mod.src:3", "/project/main.src:12"],
            ),
            "mod=1".to_owned(),
            crash(
                7,
                "/project/lib.src line 3",
                &["/project/lib.src:3", "/project/main.src:13"],
            ),
            "lib=1".to_owned(),
            crash(8, "/project/main.src line 14", &["/project/main.src:14"]),
            "main=1".to_owned(),
        ];

        assert_eq!(
            harness.terminal.print_log.borrow().join("\n"),
            expected.join("\n")
        );
    });
}

#[test]
fn should_stop_in_the_files_a_program_includes_and_imports_in_the_game() {
    in_game(env!("CARGO_TARGET_TMPDIR"), |port| async move {
        let main = "#include \"/inc.src\"\nimport_code(\"/lib.src\")\nprint(triple(2) + double(4))";
        let included = "triple = function(n)\n  return n * 3\nend function";
        let library = "double = function(n)\n  return n * 2\nend function";
        let mut harness = Harness::new(&[
            (MAIN, main),
            ("file:///project/inc.src", included),
            (LIBRARY, library),
        ]);

        for (uri, line) in [("file:///project/inc.src", 2), (LIBRARY, 2)] {
            harness
                .request_on_io(&breakpoints_request(uri, vec![breakpoint(line)]))
                .await
                .expect("set");
        }

        let mut launch = Box::pin(harness.launch_in_game(port, true));
        let mut locations = Vec::new();

        for stops in 1..=2 {
            harness.stops_on_io(stops, RUN_TIME).await;

            let frames = harness
                .request_on_io(&StackTrace {})
                .await
                .expect("a stack");
            let location = frames[0].location.clone().expect("a location");

            locations.push((location.uri, location.line));
            harness.editor.send(&Continue {}).expect("sent");
        }

        let exit = harness.wait_on_io(launch.as_mut(), RUN_TIME).await;

        assert_eq!(exit, Ok(Exit::Completion));
        assert_eq!(
            locations,
            [
                ("file:///project/inc.src".to_owned(), 2),
                (LIBRARY.to_owned(), 2)
            ]
        );
        assert_eq!(*harness.terminal.print_log.borrow(), ["14"]);
    });
}
