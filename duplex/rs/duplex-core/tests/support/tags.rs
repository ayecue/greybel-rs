use duplex_core::{Tag, TagValue};
use duplex_test_support::conformance::sensor::{Position, Reading, Shape, Unit};

/// Returns a `Reading` with every field populated.
pub fn reading() -> Reading<'static> {
    Reading {
        value: 1.5,
        station: 3,
        labels: vec!["north".to_owned()].into(),
        position: Position {
            across: 1.0,
            down: 2.0,
        },
        shape: Shape::Box {
            width: 4,
            height: 5,
        },
        unit: Unit::Kelvin,
        note: None,
    }
}

/// Returns a text tag with the given name and value.
pub fn text(name: &'static str, text: &'static str) -> Tag {
    Tag {
        name,
        value: TagValue::Text(text),
    }
}
