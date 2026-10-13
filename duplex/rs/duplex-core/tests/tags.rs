use duplex_core::{AsValueRef, FieldAccess, FieldTagContainer, Metadata, Tag, TagValue, ValueRef};
use duplex_test_support::conformance::sensor::{
    Calibrate, Kind, Position, Probe, Reading, Sensor, Shape, Silence, StationNotification, Unit,
};
use duplex_test_support::{Memory, recorder};

use crate::support::tags::{reading, text};

#[test]
fn should_keep_the_tags_of_the_protocol_and_its_messages() {
    assert_eq!(Sensor::TAGS, [text("origin", "lab")]);
    assert_eq!(
        <Reading<'_> as Metadata>::TAGS,
        [
            text("origin", "probe"),
            Tag {
                name: "number",
                value: TagValue::Integer(34)
            }
        ]
    );
    assert_eq!(
        Calibrate::TAGS,
        [Tag {
            name: "number",
            value: TagValue::Integer(1000)
        }]
    );
    assert_eq!(Silence::TAGS, []);
}

#[test]
fn should_keep_the_tags_of_a_message_s_fields_in_their_order() {
    let fields: Vec<(&str, Vec<Tag>)> = reading()
        .fields()
        .iter()
        .map(|field| (field.name, field.tags.to_vec()))
        .collect();

    assert_eq!(
        fields,
        [
            (
                "value",
                vec![
                    text("alias", "value"),
                    text("alias", "measurement"),
                    Tag {
                        name: "weight",
                        value: TagValue::Float(0.5)
                    },
                ]
            ),
            (
                "station",
                vec![
                    Tag {
                        name: "recipient",
                        value: TagValue::Flag
                    },
                    Tag {
                        name: "offset",
                        value: TagValue::Integer(-3)
                    },
                ]
            ),
            (
                "labels",
                vec![Tag {
                    name: "sorted",
                    value: TagValue::Bool(true)
                }]
            ),
            ("position", vec![]),
            ("shape", vec![]),
            ("unit", vec![]),
            ("note", vec![]),
        ]
    );
    assert_eq!(
        FieldTagContainer::with_tag(reading().fields(), "recipient").map(|field| field.name),
        Some("station")
    );
}

#[test]
fn should_keep_the_tags_of_types_variants_and_members() {
    let kind = |member: &'static str, value: u32| Tag {
        name: "kind",
        value: TagValue::Member {
            name: member,
            value,
        },
    };

    assert_eq!(Position::TAGS, [kind("Plain", 0)]);
    assert_eq!(Shape::TAGS, [kind("Sum", 1)]);
    assert_eq!(Unit::TAGS, [kind("Plain", 0)]);
    assert_eq!(Kind::TAGS, []);

    let ValueRef::Member { name, value, tags } = Unit::Celsius.as_value_ref() else {
        panic!("a member reads as a member");
    };

    assert_eq!((name, value), ("Celsius", 1));
    assert_eq!(
        tags,
        [
            text("alias", "celsius"),
            Tag {
                name: "marker",
                value: TagValue::Char('C')
            }
        ]
    );

    let ValueRef::Variant { name, tags, .. } = Shape::Point.as_value_ref() else {
        panic!("a variant reads as a variant");
    };

    assert_eq!((name, tags), ("Point", &[text("alias", "dot")][..]));
}

#[test]
#[expect(
    clippy::float_cmp,
    reason = "a float reads back exactly as it was written"
)]
fn should_read_a_message_s_fields_by_name() {
    let reading = reading();

    assert_eq!(
        reading.field("station").and_then(|value| value.integer()),
        Some(3)
    );
    assert!(matches!(reading.field("note"), Some(ValueRef::Null)));
    assert!(reading.field("missing").is_none());

    let Some(ValueRef::List(labels)) = reading.field("labels") else {
        panic!("a list reads as a list");
    };

    assert_eq!(labels[0].text(), Some("north"));

    let Some(ValueRef::Struct(position)) = reading.field("position") else {
        panic!("a struct reads as a struct");
    };

    assert!(matches!(position.field("down"), Some(ValueRef::Float(down)) if down == 2.0));

    let Some(ValueRef::Variant {
        name,
        fields,
        values,
        ..
    }) = reading.field("shape")
    else {
        panic!("a union reads as a variant");
    };

    assert_eq!(name, "Box");
    assert_eq!(fields[0].name, "width");
    assert_eq!(
        Tag::find(fields[0].tags, "recipient").map(|tag| tag.value),
        Some(TagValue::Flag)
    );
    assert_eq!(values[1].integer(), Some(5));
    assert_eq!(
        reading.field("unit").and_then(|value| value.integer()),
        Some(7)
    );
}

#[test]
fn should_receive_every_notification_a_peer_handles_as_one_type() {
    let (probe, station, queue) = Memory::connected_pair::<Probe>();
    let (received, record) = recorder();

    StationNotification::attach(&station, record);
    probe.send(&reading()).expect("the reading is sent");
    probe.send(&Silence {}).expect("the silence is sent");
    queue.deliver();

    let received = received.borrow();

    assert_eq!(
        *received,
        [
            StationNotification::Reading(reading()),
            StationNotification::Silence(Silence {})
        ]
    );
    assert_eq!(received[0].name(), "Reading");
    assert_eq!(received[0].tags(), <Reading<'_> as Metadata>::TAGS);
    assert_eq!(
        received[0].field("value").map(|value| format!("{value:?}")),
        Some("Float(1.5)".to_owned())
    );
}
