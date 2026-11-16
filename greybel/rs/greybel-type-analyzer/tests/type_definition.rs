//! Tests go to type definition.

use greybel_core::Position;

use crate::support::type_definition::type_definitions;

#[test]
fn should_go_to_the_type_tag_of_a_custom_type() {
    let source = "// @type Point\n// @property {number} x\n\n// @param {Point} p\nf = function(p)\n  print p\nend function";

    assert_eq!(
        type_definitions(source, Position::new(6, 9)),
        [(1, 1, 1, 15)]
    );
}

#[test]
fn should_go_to_the_map_literal_that_made_a_map() {
    let source = "m = {\"a\": 1}\nprint m";

    assert_eq!(
        type_definitions(source, Position::new(2, 7)),
        [(1, 5, 1, 13)]
    );
}

#[test]
fn should_go_from_an_instance_to_its_class() {
    let source = "Dog = {\"name\": \"\"}\nd = new Dog\nprint d";

    assert_eq!(
        type_definitions(source, Position::new(3, 7)),
        [(1, 7, 1, 19)]
    );
}

#[test]
fn should_go_nowhere_for_api_and_primitive_types() {
    let source = "s = get_shell\nn = 1\nprint s\nprint n";

    assert_eq!(type_definitions(source, Position::new(3, 7)), []);
    assert_eq!(type_definitions(source, Position::new(4, 7)), []);
}
