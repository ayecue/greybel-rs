//! Tests that `wait`, `time`, and `clear_screen` cooldowns use the editor transport's clock.

use std::time::Duration;

use greybel_runtime::Runtime;
use greybel_runtime::protocol::Exit;

use crate::support::editor::Exit as EditorExit;
use crate::support::host::{launch, run, setup};
use crate::support::{Harness, MAIN};

#[test]
fn should_wait_until_the_time_passed_on_the_editors_clock() {
    let mut harness = Harness::new(&[(MAIN, "t = time\nwait(5)\nprint(round(time - t))")]);

    let exit = harness.launch(false, false);

    harness.run_for(50);
    assert!(harness.terminal.print_log.borrow().is_empty());
    harness.advance(Duration::from_secs(5));

    assert_eq!(harness.wait(exit), Ok(EditorExit::Completion));
    assert_eq!(*harness.terminal.print_log.borrow(), ["5"]);
}

#[test]
fn should_clear_the_screen_again_after_a_wait() {
    let code = "for i in range(2)\n\twait(0.5)\n\tclear_screen\n\tprint(i)\nend for";
    let mut harness = Harness::new(&[(MAIN, code)]);

    let exit = harness.launch(false, false);
    let exit = harness.wait_advancing(exit, Duration::from_millis(100));

    assert_eq!(exit, Ok(EditorExit::Completion));
    assert_eq!(
        harness
            .terminal
            .print_log
            .borrow()
            .last()
            .map(String::as_str),
        Some("0")
    );
}

#[test]
fn should_terminate_a_program_while_it_waits() {
    let setup = setup("wait(60)\nprint(\"after\")", Vec::new());

    run(async move {
        let (runtime, controller) = Runtime::with_host(setup.host);

        tokio::task::spawn_local(runtime.serve());

        let exit = tokio::task::spawn_local(controller.launch(launch(false, &[])));

        tokio::time::sleep(Duration::from_millis(100)).await;
        controller.terminate();

        let exit = tokio::time::timeout(Duration::from_secs(5), exit)
            .await
            .expect("the program ends at once")
            .expect("the launch completes");

        assert!(matches!(exit, Ok(Exit::Termination)), "{exit:?}");
        assert!(setup.printed.borrow().is_empty());
    });
}
