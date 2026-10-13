use std::time::Duration;

use greybel_io::{Output, Stdout, *};
use greybel_test_support::io::line;

use crate::support::stdout_buffer::{batches, print, stdout};

#[test]
fn should_show_only_the_first_print_before_the_interval() {
    let (stdout, _time) = stdout();

    print(&stdout, "a");
    print(&stdout, "b");
    print(&stdout, "c");
    stdout.flush_if_due();
    assert_eq!(batches(&stdout), [vec![line("a")]]);
}

#[test]
fn should_show_the_waiting_prints_once_the_interval_passed() {
    let (stdout, time) = stdout();

    print(&stdout, "a");
    print(&stdout, "b");
    print(&stdout, "c");
    stdout.flush_if_due();
    time.advance(DEFAULT_INTERVAL);
    print(&stdout, "d");
    assert_eq!(
        batches(&stdout),
        [vec![line("a")], vec![line("b"), line("c"), line("d")]]
    );
}

#[test]
fn should_show_the_queue_on_a_yield_once_the_interval_passed() {
    let (stdout, time) = stdout();

    print(&stdout, "a");
    print(&stdout, "b");
    time.advance(DEFAULT_INTERVAL);
    stdout.flush_if_due();
    assert_eq!(batches(&stdout), [vec![line("a")], vec![line("b")]]);
}

#[test]
fn should_queue_prints_while_the_queue_is_under_its_capacity() {
    let (stdout, _) = stdout();
    let stdout = stdout.with_capacity(2);

    print(&stdout, "a");
    print(&stdout, "b");
    assert_eq!(batches(&stdout).len(), 1);
}

#[test]
fn should_show_a_full_queue_without_waiting() {
    let (stdout, _) = stdout();
    let stdout = stdout.with_capacity(2);

    print(&stdout, "a");
    print(&stdout, "b");
    print(&stdout, "c");
    assert_eq!(
        batches(&stdout),
        [vec![line("a")], vec![line("b"), line("c")]]
    );
}

#[test]
fn should_drop_what_a_clear_would_wipe() {
    let (stdout, _) = stdout();

    print(&stdout, "shown");
    print(&stdout, "wiped");
    stdout.clear();
    print(&stdout, "after");
    stdout.flush();
    assert_eq!(
        batches(&stdout),
        [vec![line("shown")], vec![Output::Clear, line("after")]]
    );
}

#[test]
fn should_show_every_print_with_a_zero_interval() {
    let (stdout, _) = stdout();
    let stdout = stdout.with_interval(Duration::ZERO);

    print(&stdout, "a");
    print(&stdout, "b");
    assert_eq!(batches(&stdout), [vec![line("a")], vec![line("b")]]);
}

#[test]
fn should_show_nothing_on_a_flush_with_nothing_queued() {
    let (stdout, _) = stdout();

    stdout.flush();
    assert!(batches(&stdout).is_empty());
}

#[test]
fn should_show_nothing_on_a_due_flush_with_nothing_queued() {
    let (stdout, _) = stdout();

    stdout.flush();
    stdout.flush_if_due();
    assert!(batches(&stdout).is_empty());
}
