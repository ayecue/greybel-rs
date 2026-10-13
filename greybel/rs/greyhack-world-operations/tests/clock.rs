use std::rc::Rc;
use std::time::Duration;

use duplex_test_support::local::run_local;
use greyhack_behaviour::{START_DATE, TIME_SCALE};
use greyhack_world_operations::TickOptions;
use scheduler::ManualClock;

use crate::support::clock::current_date;
use crate::support::program::{PlayerCurrentDate, ProgramThrottle};
use crate::support::{Notice, connect, server_on, settle};

/// Tests world-clock initialization and 14× advancement on the ticks, counted from the first runtime's attachment:
/// 4.5 s → 63 s, 5.5 s → 77 s, and 3,605.5 s → 14 h 1 min.
#[test]
fn should_run_the_worlds_clock_fourteen_times_as_fast_as_real_time() {
    run_local(async {
        let clock = ManualClock::new();
        let server = server_on(Rc::new(clock.clone()));
        let runtime = connect(&server);
        let session = runtime.start("/bin/date").await;
        let mut dates = Vec::new();

        for advance in [0, 4, 1, 3_600] {
            clock.advance(Duration::from_millis(advance * 1_000 + 500));
            dates.push(
                runtime
                    .channel
                    .send(&PlayerCurrentDate {
                        program: session.program,
                    })
                    .expect("the date is requested")
                    .await
                    .expect("the game server responds"),
            );
        }

        assert_eq!(dates[0], START_DATE);
        assert_eq!(
            dates[1..],
            [
                "01/Dec/2000 - 06:01".to_owned(),
                "01/Dec/2000 - 06:01".to_owned(),
                "01/Dec/2000 - 20:01".to_owned(),
            ]
        );
    });
}

/// Tests that setting the world's date moves only its clock: the ticks go on counting real time from where they were.
#[test]
fn should_go_on_ticking_after_the_date_was_set() {
    run_local(async {
        // Arrange
        let clock = ManualClock::new();
        let server = server_on(Rc::new(clock.clone()));

        server.tick().await;
        server
            .change_world(async |world| world.set_date("04/Jan/2000 - 06:00"))
            .await;

        let start = server.world().await.time_seconds();

        // Act
        clock.advance(Duration::from_secs(1));
        server.tick().await;

        // Assert
        assert_eq!(server.world().await.time_seconds() - start, TIME_SCALE);
    });
}

/// Tests that a late wake runs at most one real second's ticks, the clock jumping over the others.
#[test]
fn should_jump_over_the_ticks_a_late_wake_missed() {
    run_local(async {
        // Arrange
        let clock = ManualClock::new();
        let server = server_on(Rc::new(clock.clone()));

        server.tick().await;

        let start = server.world().await.time_seconds();

        // Act
        clock.advance(Duration::from_secs(60));
        server.tick().await;

        // Assert
        assert_eq!(server.world().await.time_seconds() - start, 60 * TIME_SCALE);
    });
}

/// Tests that a program tree that had to wait for its operation budget while another tree wanted the world is told to
/// pause, after its operation's response, for one tick; a tree whose operations didn't wait isn't.
#[test]
fn should_throttle_a_tree_that_waited_for_its_budget() {
    run_local(async {
        // Arrange
        let clock = ManualClock::new();
        let server = server_on(Rc::new(clock.clone()));

        server.set_tick_options(TickOptions {
            operations: 1,
            operation_cap: 1,
            ..TickOptions::default()
        });

        let (busy, other) = (connect(&server), connect(&server));
        let busy_program = busy.start("/bin/busy").await.program;
        let other_program = other.start("/bin/other").await.program;

        current_date(&busy, busy_program).await;

        let world = server.world().await;
        let waiting = current_date(&other, other_program);

        settle().await;

        // Act
        let held = current_date(&busy, busy_program);

        settle().await;
        drop(world);
        clock.advance(Duration::from_millis(100));
        held.await;
        waiting.await;
        settle().await;

        // Assert
        let throttle = Notice::Throttle(ProgramThrottle {
            program: busy_program,
            milliseconds: 72,
        });

        assert_eq!(busy.notices.borrow().as_slice(), [throttle]);
        assert_eq!(other.notices.borrow().as_slice(), []);
    });
}

/// The daily upkeep runs once the world's clock crosses a day boundary: a coin's price is recalculated from its trades.
#[test]
fn should_run_the_daily_upkeep_when_a_world_day_passes() {
    run_local(async {
        let clock = ManualClock::new();
        let server = server_on(Rc::new(clock.clone()));
        let runtime = connect(&server);
        let _session = runtime.start("/bin/date").await;

        let (coin, before) = server
            .change_world(async |world| {
                let coin = world.coins().await.unwrap().swap_remove(0);

                world.record_trade(coin.id, 500, 1).await.unwrap();

                (coin.id, coin.price)
            })
            .await;

        // A world day is 86,400 world seconds, so 86,400 / TIME_SCALE real seconds.
        clock.advance(Duration::from_secs(86_400 / TIME_SCALE + 10));
        settle().await;
        settle().await;
        settle().await;

        let after = server
            .change_world(async |world| world.coin(coin).await.unwrap().unwrap().price)
            .await;

        assert_ne!(after, before, "the daily upkeep recalculated the price");
    });
}
