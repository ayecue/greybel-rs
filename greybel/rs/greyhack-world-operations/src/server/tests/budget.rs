#[path = "support/budget.rs"]
mod support;

use std::pin::pin;

use futures_util::FutureExt;
use greybel_test_support::future::block_on;
use support::{OTHER, TREE};

use super::{BudgetTable, Owner};

#[test]
fn should_let_a_tree_alone_draw_beyond_its_budget() {
    // Arrange
    let table = BudgetTable::default();
    let _claim = table.claim(Owner::Tree(TREE));

    // Act
    let first = pin!(table.draw(TREE, 0)).now_or_never();
    let second = pin!(table.draw(TREE, 0)).now_or_never();

    // Assert
    assert_eq!((first, second), (Some(()), Some(())));
}

#[test]
fn should_hold_back_a_tree_without_tokens_while_the_world_runs_routines() {
    block_on(async {
        // Arrange
        let table = BudgetTable::default();
        let _routines = table.claim(Owner::World);

        table.draw(TREE, 1).await;

        let mut draw = pin!(table.draw(TREE, 1));

        // Act
        let before = draw.as_mut().now_or_never();

        table.refill(1, 1, 1);

        let after = draw.as_mut().now_or_never();

        // Assert
        assert_eq!(before, None);
        assert_eq!(after, Some(()));
    });
}

#[test]
fn should_hold_back_a_tree_without_tokens_while_another_tree_waits() {
    block_on(async {
        // Arrange
        let table = BudgetTable::default();
        let _other = table.claim(Owner::Tree(OTHER));

        table.draw(TREE, 1).await;

        // Act
        let held = pin!(table.draw(TREE, 1)).now_or_never();

        // Assert
        assert_eq!(held, None);
    });
}

#[test]
fn should_save_up_no_more_tokens_than_the_cap() {
    block_on(async {
        // Arrange
        let table = BudgetTable::default();
        let _routines = table.claim(Owner::World);

        table.draw(TREE, 2).await;
        table.refill(10, 32, 2);

        // Act
        table.draw(TREE, 2).await;
        table.draw(TREE, 2).await;

        let held = pin!(table.draw(TREE, 2)).now_or_never();

        // Assert
        assert_eq!(held, None);
    });
}

#[test]
fn should_throttle_a_tree_once_after_it_waited_for_a_refill() {
    block_on(async {
        // Arrange
        let table = BudgetTable::default();
        let _routines = table.claim(Owner::World);

        table.draw(TREE, 1).await;

        let unthrottled = table.take_throttle(TREE);
        let mut draw = pin!(table.draw(TREE, 1));

        let _ = draw.as_mut().now_or_never();

        table.refill(1, 1, 1);
        draw.await;

        // Act
        let throttles = (table.take_throttle(TREE), table.take_throttle(TREE));

        // Assert
        assert!(!unthrottled);
        assert_eq!(throttles, (true, false));
    });
}

#[test]
fn should_not_throttle_a_tree_alone_beyond_its_budget() {
    // Arrange
    let table = BudgetTable::default();
    let _claim = table.claim(Owner::Tree(TREE));

    let _ = pin!(table.draw(TREE, 0)).now_or_never();

    // Act
    let throttled = table.take_throttle(TREE);

    // Assert
    assert!(!throttled);
}
