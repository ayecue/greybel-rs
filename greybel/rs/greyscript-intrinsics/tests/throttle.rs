//! Tests that a runtime pauses a program tree the game server throttled.

use std::cell::Cell;
use std::rc::Rc;

use duplex_test_support::local::run_local;
use greybel_interpreter::InterpreterOptions;
use greybel_test_support::interpreter::{inline, snapshot_options};
use greyscript_intrinsics::GreyHackInterpreter;
use scheduler::ManualClock;

use crate::support::throttle::{pass, settle, throttling_server};

/// The game server throttles the program for 500 ms as it starts: it prints nothing before its clock passed them.
#[test]
fn should_pause_a_throttled_program_tree_on_its_clock() {
    run_local(async {
        // Arrange
        let clock = ManualClock::new();
        let (stdout, options) = snapshot_options();
        let options = InterpreterOptions {
            clock: Rc::new(clock.clone()),
            ..options
        };
        let server = throttling_server(500);
        let mut program = GreyHackInterpreter::start(&server, "player", options, inline("print 1"))
            .await
            .expect("the program starts");
        let finished = Rc::new(Cell::new(None));
        let run = Rc::clone(&finished);

        tokio::task::spawn_local(async move { run.set(Some(program.run().await.is_ok())) });

        // Act
        settle().await;
        pass(&clock, 450).await;

        let paused = (stdout.text(), finished.get());

        pass(&clock, 1_000).await;

        let resumed = (stdout.text(), finished.get());

        // Assert
        assert_eq!(paused, (String::new(), None));
        assert_eq!(resumed, ("1".to_owned(), Some(true)));
    });
}
