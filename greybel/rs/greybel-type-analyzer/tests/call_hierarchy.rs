//! Tests the call hierarchy.

use greybel_core::Position;

use crate::support::call_hierarchy::{incoming, list_callers_in, outgoing, prepared};

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
fn should_list_the_callers_of_a_function_stored_in_a_map() {
    let source = "App = {}
App.format = function(x)
  return x
end function
run = function()
  return App.format(1)
end function
App.format(2)";

    assert_eq!(
        list_callers_in(source, Position::new(2, 6)),
        [
            ("run".to_owned(), vec![(6, 14)]),
            ("/main.src".to_owned(), vec![(8, 5)]),
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
