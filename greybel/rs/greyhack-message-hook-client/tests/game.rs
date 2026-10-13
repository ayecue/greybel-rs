//! End-to-end tests against headless Grey Hack, its message-hook plugin, proxy, and interpreter.

use std::time::Duration;

use greybel_source_map::{Location, SourceMap};
use greybel_test_support::game::Game;
use greyhack_message_hook_client::protocol::game_version;
use greyhack_message_hook_client::{
    BuildOutcome, BuildState, ContextBreakpoint, DebugEntry, DebugFrame, Evaluation,
    GreybelNotification, Payload, ScriptType, StepMode,
};

use crate::support::game::{
    FILEPATH, disposal, resume_to_end, run_to_end, script, stop, stopping_context,
};

#[tokio::test]
async fn should_report_a_single_player_game_on_a_health_check() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let status = client.health_check().await.unwrap();

    client.shutdown().await;
    assert!(status.has_player_client);
    assert!(status.is_single_player);
    assert_eq!(status.game_version, game_version());
}

#[tokio::test]
async fn should_print_what_a_script_prints_on_the_games_computer() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let context = client
        .create_context(script(
            "print(\"hello \" + (1 + 1))\nprint(typeof(get_shell.host_computer))",
        ))
        .await
        .unwrap();
    let run = run_to_end(&context).await;

    client.shutdown().await;
    assert_eq!(run.output(), "hello 2\ncomputer");
    assert!(!run.finished.failed);
}

#[tokio::test]
async fn should_name_the_files_a_script_was_built_from_by_its_source_map() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let mut options = script("x = 1\ny = [1][5]");

    options.source_map = SourceMap::of_lines([
        Some(Location::new("/project/main.src", 1)),
        Some(Location::new("/project/lib.src", 7)),
    ]);

    let context = client.create_context(options).await.unwrap();
    let run = run_to_end(&context).await;

    client.shutdown().await;
    assert!(
        run.output()
            .contains("Runtime Error: Index Error: list index (5) out of range (-1 to 0) [/project/lib.src line 7]"),
        "{}",
        run.output()
    );
    assert!(run.finished.failed);
}

/// In strict GreyScript, as in the game, `debugger` is a name; elsewhere a `debugger` statement stops the script.
#[tokio::test]
async fn should_read_debugger_as_a_name_in_strict_mode() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let mut options = script("debugger = 2\nprint debugger");

    options.strict_mode = true;

    let context = client.create_context(options).await.unwrap();
    let run = run_to_end(&context).await;

    client.shutdown().await;
    assert_eq!(run.output(), "2");
    assert!(!run.finished.failed);
}

#[tokio::test]
async fn should_report_a_script_that_fails() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let context = client
        .create_context(script("print(1)\nexit(nope.length)"))
        .await
        .unwrap();
    let run = run_to_end(&context).await;

    client.shutdown().await;

    let output = run.output();

    assert!(output.starts_with("1\n"), "output {output}");
    assert!(
        output.contains("Undefined Identifier: 'nope'"),
        "output {output}"
    );
    assert!(run.finished.failed, "output {output}");
}

#[tokio::test]
async fn should_stop_at_a_breakpoint_with_the_scripts_variables() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let (context, mut events) = stopping_context(&client, "x = 41\nx = x + 1\nprint(x)", 2).await;
    let breakpoint = stop(&mut events).await;
    let state = context.runtime_state().await.unwrap();
    let run = resume_to_end(&context).await;

    client.shutdown().await;
    assert_eq!(breakpoint.line_num, 2);
    assert_eq!(breakpoint.filepath, FILEPATH);
    assert!(
        state.variables.to_string().contains("41"),
        "variables {}",
        state.variables
    );
    assert_eq!(run.output(), "42");
}

#[tokio::test]
async fn should_step_into_the_next_line() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let (context, mut events) = stopping_context(&client, "x = 1\nx = x + 1\nprint(x)", 1).await;
    let first = stop(&mut events).await;

    context.step(StepMode::Into).await.unwrap();

    let second = stop(&mut events).await;
    let run = resume_to_end(&context).await;

    client.shutdown().await;
    assert_eq!((first.line_num, second.line_num), (1, 2));
    assert_eq!(run.output(), "2");
}

#[tokio::test]
async fn should_run_code_injected_at_a_breakpoint() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let (context, mut events) = stopping_context(&client, "x = 1\nx = x + 1\nprint(x)", 2).await;

    stop(&mut events).await;
    context.inject_code("x = 10").await.unwrap();
    stop(&mut events).await;

    let run = resume_to_end(&context).await;

    client.shutdown().await;
    assert_eq!(run.output(), "11");
}

#[tokio::test]
async fn should_dispose_a_context_as_soon_as_it_is_created() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let context = client
        .create_context(script("while true\n  wait(0.1)\nend while"))
        .await
        .unwrap();
    let mut events = context.events();

    context.dispose().await.unwrap();

    let disposed = disposal(&mut events).await;

    client.shutdown().await;
    assert_eq!(disposed.context_id, context.id());
}

#[tokio::test]
async fn should_dispose_a_running_context() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let context = client
        .create_context(script(
            "print(\"running\")\nwhile true\n  wait(0.1)\nend while",
        ))
        .await
        .unwrap();
    let mut events = context.events();

    while !matches!(
        events.next().await,
        Some(GreybelNotification::PrintSentClientRpc(_))
    ) {}

    context.dispose().await.unwrap();

    let disposed = disposal(&mut events).await;

    client.shutdown().await;
    assert_eq!(disposed.context_id, context.id());
}

#[tokio::test]
async fn should_build_resources_into_new_folders_and_run_the_build_scripts_in_the_game() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;

    let folder = format!("/root/build{}", std::process::id());
    let build = client.create_build(&folder, &folder, true).await.unwrap();

    build
        .add_resource("/lib/hello.src", "print(\"hello\")")
        .await
        .unwrap();

    let check = format!("print(typeof(get_shell.host_computer.File(\"{folder}/lib/hello.src\")))");

    build.add_script(ScriptType::Post, &check).await.unwrap();

    let outcome = build
        .perform_and_wait(Some(Duration::from_secs(120)))
        .await
        .unwrap();
    let disposed = build.dispose().await.unwrap();

    client.shutdown().await;
    assert!(
        matches!(&outcome, BuildOutcome::Completion(_)),
        "outcome {outcome:?}"
    );
    assert_eq!(
        outcome.state(),
        BuildState::Complete,
        "error {}",
        outcome.error_message()
    );
    assert!(
        outcome.output().contains("file"),
        "output {}",
        outcome.output()
    );
    assert!(disposed.dispose_success);
}

#[tokio::test]
async fn should_step_over_into_and_out_of_calls_with_each_frames_variables() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let source =
        "f = function(x)\n  y = x * 2\n  return y\nend function\na = f(1)\nb = f(a)\nprint(b)";
    let (context, mut events) = stopping_context(&client, source, 5).await;
    let mut lines = vec![stop(&mut events).await.line_num];

    context.step(StepMode::Over).await.unwrap();
    lines.push(stop(&mut events).await.line_num);
    context.step(StepMode::Into).await.unwrap();

    let inside = stop(&mut events).await;

    lines.push(inside.line_num);
    context.step(StepMode::Over).await.unwrap();
    lines.push(stop(&mut events).await.line_num);
    context.step(StepMode::Out).await.unwrap();
    lines.push(stop(&mut events).await.line_num);

    let run = resume_to_end(&context).await;

    client.shutdown().await;
    assert_eq!(lines, [5, 6, 2, 3, 7]);

    let frames: Vec<DebugFrame> = inside.frames.json().expect("the frames");
    let names: Vec<(&str, i32)> = frames
        .iter()
        .map(|frame| (frame.name.as_str(), frame.line_num))
        .collect();

    assert_eq!(names, [("function test.src:2", 2), ("test.src", 6)]);

    let locals: Vec<(&str, &str)> = frames[0]
        .locals
        .iter()
        .map(|local| (local.name.as_str(), local.value.text.as_str()))
        .collect();

    assert_eq!(locals, [("x", "2")]);
    assert!(
        frames[1]
            .locals
            .iter()
            .any(|local| local.name == "a" && local.value.text == "2")
    );
    assert_eq!(run.output(), "4");
}

#[tokio::test]
async fn should_show_the_items_of_lists_and_maps() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let source = "data = {\"name\": \"neo\", \"items\": [1, [2]]}\nprint(data.name)";
    let (context, mut events) = stopping_context(&client, source, 2).await;
    let stopped = stop(&mut events).await;

    resume_to_end(&context).await;
    client.shutdown().await;

    let globals: Vec<DebugEntry> = stopped.globals.json().expect("the globals");
    let data = globals
        .iter()
        .find(|entry| entry.name == "data")
        .expect("data");

    assert_eq!(data.value.type_name, "map");

    let entries: Vec<(&str, Option<&str>, &str)> = data
        .value
        .children
        .iter()
        .map(|entry| {
            (
                entry.name.as_str(),
                entry.accessor.as_deref(),
                entry.value.text.as_str(),
            )
        })
        .collect();

    assert_eq!(
        entries,
        [
            ("name", Some("[\"name\"]"), "\"neo\""),
            ("items", Some("[\"items\"]"), "[1, [2]]")
        ]
    );

    let nested = &data.value.children[1].value.children[1].value;

    assert_eq!(
        (
            nested.type_name.as_str(),
            nested.children[0].value.text.as_str()
        ),
        ("list", "2")
    );
}

#[tokio::test]
async fn should_stop_at_a_breakpoint_set_while_the_program_runs() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let mut options = script("wait(2)\nprint(1)\nprint(2)");

    options.debug_mode = true;

    let context = client.create_context(options).await.unwrap();
    let mut events = context.events();

    context
        .set_breakpoints(&[ContextBreakpoint {
            filepath: FILEPATH.to_owned(),
            line_num: 3,
        }])
        .await
        .unwrap();

    // The game sends the program's prints in batches, so the first one can arrive before or after the stop: one
    // subscription reads them all.
    let mut prints = Vec::new();
    let mut stopped = None;

    loop {
        match events.next().await.expect("the context runs") {
            GreybelNotification::PrintSentClientRpc(print) => prints.push(print.zip_output.text()),
            GreybelNotification::ContextBreakpointRpc(breakpoint) => {
                stopped = Some(breakpoint.line_num);
                context
                    .set_debug_mode(false)
                    .await
                    .expect("the debug mode is turned off");
            }
            GreybelNotification::FinishedContextRpc(_) => break,
            _ => {}
        }
    }

    client.shutdown().await;
    assert_eq!(stopped, Some(3));
    assert_eq!(prints.join("\n"), "1\n2");
}

#[tokio::test]
async fn should_evaluate_code_where_the_program_stopped() {
    let (_game, client) = Game::start_with_player(env!("CARGO_TARGET_TMPDIR")).await;
    let source = "f = function(n)\n  return n * 2\nend function\nx = 1\nprint(x)";
    let (context, mut events) = stopping_context(&client, source, 5).await;

    stop(&mut events).await;

    let value = context.evaluate("f(x) * 21").await.unwrap();
    let statements = context.evaluate("x = 5").await.unwrap();
    let invalid = context.evaluate("1 +").await.unwrap();
    let raised = context.evaluate("undefined_name + 1").await.unwrap();
    let after = context.evaluate("x").await.unwrap();
    let run = resume_to_end(&context).await;

    client.shutdown().await;

    let Evaluation::Value(value) = value else {
        panic!("expected a value, got {value:?}");
    };

    assert_eq!(
        (value.type_name.as_str(), value.text.as_str()),
        ("number", "42")
    );
    assert_eq!(statements, Evaluation::Statements);
    assert!(matches!(invalid, Evaluation::Failure(_)), "{invalid:?}");
    assert!(
        matches!(&raised, Evaluation::Failure(message) if message.contains("undefined_name")),
        "{raised:?}"
    );
    assert!(
        matches!(&after, Evaluation::Value(value) if value.text == "5"),
        "{after:?}"
    );
    assert_eq!(run.output(), "5");
}
