#[path = "support/nodes_by_line.rs"]
mod support;

use support::{containing, id, ranges};

use super::*;

#[test]
fn should_find_exactly_the_ranges_containing_a_line() {
    let items = ranges();
    let index = NodesByLine::new(items.iter().copied());
    let cases: Vec<_> = (0..=170)
        .map(|line| (line, containing(&items, line)))
        .collect();

    let outcomes: Vec<_> = cases
        .iter()
        .map(|(line, _)| {
            let mut actual = index.get(*line);

            actual.sort();

            actual
        })
        .collect();

    for ((line, expected), actual) in cases.into_iter().zip(outcomes) {
        assert_eq!(actual, expected, "line {line}");
    }
}

#[test]
fn should_order_results_from_the_most_specific_tree_level_up() {
    let index = NodesByLine::new([(1, 8, id(0)), (5, 5, id(1)), (4, 7, id(2)), (5, 6, id(3))]);

    let found = index.get(5);

    assert_eq!(found, vec![id(1), id(3), id(0), id(2)]);
}

#[test]
fn should_find_nothing_in_an_empty_index() {
    let index = NodesByLine::new([]);

    let found = index.get(1);

    assert!(found.is_empty());
}
