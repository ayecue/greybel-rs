//! Tests the call hierarchy.

use greybel_core::Position;

use crate::support::call_hierarchy::{incoming, outgoing, prepared};

#[test]
fn should_prepare_the_function_at_its_name_or_a_reference() {
    assert_eq!(
        prepared(Position::new(1, 3)),
        [("helper".to_owned(), (1, 1))]
    );
    assert_eq!(prepared(Position::new(10, 2)), [("run".to_owned(), (4, 1))]);
    assert_eq!(prepared(Position::new(5, 4)), []);
}

#[test]
fn should_list_the_callers_of_a_function() {
    assert_eq!(
        incoming(Position::new(1, 3)),
        [
            ("run".to_owned(), vec![(5, 7), (6, 7)]),
            ("/main.src".to_owned(), vec![(11, 1)]),
        ]
    );
}

#[test]
fn should_list_the_functions_a_function_calls() {
    assert_eq!(
        outgoing(Position::new(4, 2)),
        [("helper".to_owned(), vec![(5, 7), (6, 7)])]
    );
}
