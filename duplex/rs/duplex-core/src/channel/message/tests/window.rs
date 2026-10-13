#[path = "support/window.rs"]
mod support;

use support::grants;

use super::{INITIAL_CREDIT, ReceiveWindow};

#[test]
fn should_grant_its_window_past_the_initial_credit_as_its_stream_starts() {
    let mut window = ReceiveWindow::new(32, Some(8));

    let granted = window.start();

    assert_eq!(granted, Some(32 - INITIAL_CREDIT));
}

#[test]
fn should_grant_what_it_took_once_it_took_half_its_window() {
    let mut window = ReceiveWindow::new(32, Some(8));

    let granted = grants(&mut window, 64, 8);

    assert_eq!(granted, vec![31, 16, 16, 16, 16]);
}

#[test]
fn should_fit_its_window_to_the_budget_at_the_declared_item_size() {
    let mut window = ReceiveWindow::new(4096, Some(64 * 1024));

    let granted = grants(&mut window, 16, 64 * 1024);

    assert_eq!(granted, vec![15, 8, 8]);
}

#[test]
fn should_shrink_its_window_to_the_largest_item_it_received() {
    let mut window = ReceiveWindow::new(4096, None);

    let granted = grants(&mut window, 256, 256 * 1024);

    assert_eq!(granted[0], 255);
    assert_eq!(&granted[1..], &[2, 2]);
}

#[test]
fn should_grow_its_window_for_small_items_up_to_its_most() {
    let mut window = ReceiveWindow::new(4096, None);

    let granted = grants(&mut window, 1, 4);

    assert_eq!(granted, vec![255, 4096 - 255]);
}

#[test]
fn should_grant_once_every_item_was_taken_with_a_window_of_one() {
    let mut window = ReceiveWindow::new(4096, Some(2 * 1024 * 1024));

    let granted = grants(&mut window, 3, 2 * 1024 * 1024);

    assert_eq!(granted, vec![1, 1, 1]);
}
