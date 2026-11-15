use std::rc::Rc;

use greyhack_world_effects::{QUEUE_TABLES, Routine, Scheduler};
use greyhack_world_storage::sql::{execute, fetch_value};
use turso::params;

use crate::support::scheduler::{count, greet, run_all, window};
use crate::support::{Counter, Greeting, host, scheduler};

#[tokio::test]
async fn should_run_routines_due_in_the_same_second_by_id() {
    // Arrange
    let mut host = host().await;
    let scheduler = scheduler(&host);

    let first = greet(&scheduler, &host, 10, "first").await;
    let second = greet(&scheduler, &host, 10, "second").await;
    let third = greet(&scheduler, &host, 10, "third").await;

    // Act
    let output = run_all(&scheduler, &mut host, 10).await;

    // Assert
    assert!(first < second && second < third);
    assert_eq!(output, ["first", "second", "third"]);
}

#[tokio::test]
async fn should_run_routines_by_due_second_before_id() {
    // Arrange
    let mut host = host().await;
    let scheduler = scheduler(&host);

    greet(&scheduler, &host, 30, "late id, early").await;
    greet(&scheduler, &host, 20, "early id, early").await;
    greet(&scheduler, &host, 20, "early id, late").await;

    // Act
    let output = run_all(&scheduler, &mut host, 30).await;

    // Assert
    assert_eq!(
        output,
        ["early id, early", "early id, late", "late id, early"]
    );
}

#[tokio::test]
async fn should_run_a_lower_kind_code_first_when_equally_late() {
    // Arrange
    let mut host = host().await;
    let scheduler = scheduler(&host);

    count(&scheduler, &host, 10, 7).await;
    greet(&scheduler, &host, 10, "hello").await;

    // Act
    let output = run_all(&scheduler, &mut host, 10).await;

    // Assert
    assert_eq!(output, ["hello", "7"]);
}

#[tokio::test]
async fn should_raise_the_priority_of_late_routines() {
    // Arrange
    let mut host = host().await;
    let scheduler = scheduler(&host);

    // Five raise periods late: priority 5 - 5 = 0, which ties with a fresh greeting and wins by
    // its earlier second.
    count(&scheduler, &host, 0, 1).await;
    greet(&scheduler, &host, 300, "fresh").await;

    // Act
    let output = run_all(&scheduler, &mut host, 300).await;

    // Assert
    assert_eq!(output, ["1", "fresh"]);
}

#[tokio::test]
async fn should_not_run_routines_that_are_not_due() {
    // Arrange
    let mut host = host().await;
    let scheduler = scheduler(&host);

    greet(&scheduler, &host, 11, "later").await;

    // Act
    let output = run_all(&scheduler, &mut host, 10).await;
    let next = scheduler.next_due_at(&host.conn).await.unwrap();

    // Assert
    assert!(output.is_empty());
    assert_eq!(next, Some(11));
}

#[tokio::test]
async fn should_dispatch_each_routine_to_the_kind_of_its_code() {
    // Arrange
    let mut host = host().await;
    let scheduler = scheduler(&host);

    count(&scheduler, &host, 1, 42).await;
    greet(&scheduler, &host, 2, "hi").await;

    // Act
    run_all(&scheduler, &mut host, 10).await;

    // Assert
    assert_eq!(host.log(), ["greet hi with 1 queued", "count 42"]);
}

#[tokio::test]
async fn should_remove_the_queue_row_before_the_handler_runs() {
    // Arrange
    let mut host = host().await;
    let scheduler = scheduler(&host);

    greet(&scheduler, &host, 1, "only").await;

    // Act
    run_all(&scheduler, &mut host, 1).await;

    // Assert
    assert_eq!(host.log(), ["greet only with 0 queued"]);
}

#[tokio::test]
async fn should_remove_a_routine_whose_handler_failed() {
    // Arrange
    let host = host().await;
    let scheduler = scheduler(&host);
    let conn = host.conn.clone();

    greet(&scheduler, &host, 1, "fail").await;

    // Act
    let result = scheduler.run_next_due(&conn, window(1)).await;

    // Assert
    assert_eq!(result.unwrap_err().to_string(), "the greeting failed");
    assert_eq!(scheduler.next_due_at(&conn).await.unwrap(), None);
}

#[tokio::test]
async fn should_cancel_a_routine_with_its_data() {
    // Arrange
    let mut host = host().await;
    let scheduler = scheduler(&host);

    let cancelled = greet(&scheduler, &host, 1, "cancelled").await;

    greet(&scheduler, &host, 1, "kept").await;

    // Act
    let removed = scheduler.cancel(&host.conn, cancelled).await.unwrap();
    let again = scheduler.cancel(&host.conn, cancelled).await.unwrap();
    let rows = fetch_value::<i64>(&host.conn, "SELECT COUNT(*) FROM routine_greetings", ())
        .await
        .unwrap();
    let output = run_all(&scheduler, &mut host, 1).await;

    // Assert
    assert!(removed);
    assert!(!again);
    assert_eq!(rows, Some(1));
    assert_eq!(output, ["kept"]);
}

#[tokio::test]
async fn should_delete_a_routine_whose_data_is_missing() {
    // Arrange
    let mut host = host().await;
    let scheduler = scheduler(&host);

    greet(&scheduler, &host, 1, "orphan").await;
    greet(&scheduler, &host, 2, "kept").await;
    execute(
        &host.conn,
        "DELETE FROM routine_greetings WHERE text = ?1",
        params!["orphan"],
    )
    .await
    .unwrap();

    // Act
    let output = run_all(&scheduler, &mut host, 5).await;
    let queued = fetch_value::<i64>(&host.conn, "SELECT COUNT(*) FROM routines", ())
        .await
        .unwrap();

    // Assert
    assert_eq!(output, ["kept"]);
    assert_eq!(queued, Some(0));
}

#[tokio::test]
async fn should_delete_a_routine_of_an_unregistered_kind_when_selected() {
    // Arrange
    let mut host = host().await;
    let scheduler = scheduler(&host);

    execute(
        &host.conn,
        "INSERT INTO routines (at, kind) VALUES (1, 'unknown')",
        (),
    )
    .await
    .unwrap();
    greet(&scheduler, &host, 2, "known").await;

    // Act
    let listed = scheduler.entries(&host.conn).await.unwrap().len();
    let output = run_all(&scheduler, &mut host, 5).await;
    let queued = fetch_value::<i64>(&host.conn, "SELECT COUNT(*) FROM routines", ())
        .await
        .unwrap();

    // Assert
    assert_eq!(listed, 1);
    assert_eq!(output, ["known"]);
    assert_eq!(queued, Some(0));
}

#[tokio::test]
async fn should_list_entries_with_their_data_in_due_order() {
    // Arrange
    let host = host().await;
    let scheduler = scheduler(&host);

    let late = greet(&scheduler, &host, 9, "late").await;
    let count_id = count(&scheduler, &host, 3, 8).await;
    let early = greet(&scheduler, &host, 3, "early").await;

    // Act
    let entries = scheduler.entries(&host.conn).await.unwrap();

    // Assert
    let summary: Vec<_> = entries
        .iter()
        .map(|entry| (entry.id, entry.at, entry.key))
        .collect();

    assert_eq!(
        summary,
        [
            (count_id, 3, Counter::KEY),
            (early, 3, Greeting::KEY),
            (late, 9, Greeting::KEY)
        ]
    );
    assert_eq!(entries[0].data::<i64>(), Some(&8));
    assert_eq!(entries[0].data::<String>(), None);
    assert_eq!(
        entries[1].data::<String>().map(String::as_str),
        Some("early")
    );
    assert_eq!(
        entries
            .into_iter()
            .nth(2)
            .unwrap()
            .into_data::<String>()
            .as_deref(),
        Some("late")
    );
}

#[tokio::test]
async fn should_run_at_most_the_budget_and_call_back_after_each() {
    // Arrange
    let host = host().await;
    let scheduler = scheduler(&host);
    let conn = host.conn.clone();

    greet(&scheduler, &host, 1, "a").await;
    greet(&scheduler, &host, 2, "b").await;
    greet(&scheduler, &host, 3, "c").await;

    // Act
    let ran = scheduler
        .run_due(&conn, window(10), 2, |entry| {
            let mut state = host.state.borrow_mut();

            state.log.push(format!("after {}", entry.id.get()));
            state.output.push("called".to_owned());
        })
        .await
        .unwrap();

    // Assert
    assert_eq!(ran, 2);
    assert_eq!(host.state.borrow().output, ["a", "called", "b", "called"]);
    assert_eq!(scheduler.next_due_at(&conn).await.unwrap(), Some(3));
}

#[tokio::test]
async fn should_report_the_earliest_due_second() {
    // Arrange
    let host = host().await;
    let scheduler = scheduler(&host);

    assert_eq!(scheduler.next_due_at(&host.conn).await.unwrap(), None);

    greet(&scheduler, &host, 50, "b").await;
    greet(&scheduler, &host, 20, "a").await;

    // Act
    let next = scheduler.next_due_at(&host.conn).await.unwrap();

    // Assert
    assert_eq!(next, Some(20));
}

#[tokio::test]
async fn should_list_the_tables_of_the_registered_kinds_in_registration_order() {
    // Arrange
    let host = host().await;
    let scheduler = scheduler(&host);

    // Act
    let tables: Vec<_> = scheduler
        .kind_tables()
        .iter()
        .flat_map(|set| set.tables())
        .copied()
        .collect();

    // Assert
    assert_eq!(tables, ["routine_greetings", "routine_counters"]);
    assert_eq!(QUEUE_TABLES.tables(), ["routines"]);
}

#[tokio::test]
#[should_panic(expected = "routine kind key test.greeting is registered twice")]
async fn should_reject_a_kind_registered_twice() {
    // Arrange
    let host = host().await;
    let scheduler = scheduler(&host);

    // Act
    scheduler.register(Rc::new(Greeting::new(&host)));
}

#[tokio::test]
#[should_panic(expected = "routine kind key test.greeting is not registered")]
async fn should_reject_scheduling_an_unregistered_kind() {
    // Arrange
    let scheduler = Scheduler::new();
    let host = host().await;

    // Act
    let _ = scheduler
        .schedule::<Greeting>(&host.conn, 1, &"x".to_owned())
        .await;
}
