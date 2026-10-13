use duplex_core::{
    BinaryDecode, BinaryEncode, BinaryReader, BinaryWriter, BorrowDecode, ChannelOptions,
    DecodeFailure, Error, ErrorCode, Role, ValidatedDecode, Validation, Varuint,
};
use duplex_test_support::Memory;
use duplex_test_support::binary::{bytes, bytes_at, decode, hex, unhex};
use duplex_test_support::conformance::calculator::{
    Add, Divide, DivisionFailure, Log, Phase, ProgressRef, Step, StepRef, User,
};
use duplex_test_support::conformance::credentials;
use duplex_test_support::conformance::layouts::{
    CharacterLayout, DefaultLayout, Example, Flags, GREETING, NumberLayout, PRECISION, Shape,
    Signal,
};
use duplex_test_support::conformance::login::Member;
use duplex_test_support::fixtures::Fixtures;
use duplex_test_support::frames::{Frame, Slice};
use duplex_test_support::future::ready_output;

use crate::support::binary::{
    ALIASED_BATCH, BATCH, FAILURE_BATCH, MALFORMED_TEXT_BODY, OPEN_BATCH, body_values, example,
    flags, keywords, layout_values, printed, progress, raw, round_trip, shapes, signals,
};
use crate::support::security::SECURE_FIXTURES;
use crate::support::{log, logs};

#[test]
fn should_lay_out_fields_after_their_flags_in_their_order() {
    let value = example();
    let written = bytes(&value);

    assert_eq!(written, [1, 4, b't', b'e', b's', b't', 0xac, 0x02]);
    round_trip(&value);
}

#[test]
fn should_write_integers_as_varints() {
    let numbers = NumberLayout {
        u8: 255,
        u16: 1,
        u32: 128,
        u64: u64::MAX,
        i8: -1,
        i16: -1,
        i32: 1,
        i64: i64::MIN,
        f32: -1.25,
        f64: 0.1,
    };
    let written = bytes(&numbers);

    assert_eq!(&written[..5], [255, 1, 0x80, 1, 0xff]);
    assert_eq!(written.len(), 1 + 1 + 2 + 10 + 1 + 1 + 1 + 10 + 4 + 8);
    round_trip(&numbers);
}

#[test]
fn should_size_varuints_as_they_are_encoded() {
    let values = [0, 127, 128, 16_383, 16_384, u64::from(u32::MAX), u64::MAX];

    for value in values {
        let encoded = Varuint::new(value);
        let mut writer = BinaryWriter::new();

        writer.varuint(value);

        assert_eq!(encoded.as_bytes(), writer.into_bytes(), "{value}");
        assert_eq!(Varuint::size(value), encoded.as_bytes().len(), "{value}");
    }

    assert_eq!(Varuint::new(16_384).as_bytes(), [0x80, 0x80, 1]);
    assert_eq!(Varuint::new(u64::MAX).as_bytes().len(), Varuint::MAX_BYTES);
}

#[test]
fn should_zigzag_signed_integers() {
    let written = [-1_i64, 1, -64, 64].map(|value| bytes(&value));

    assert_eq!(written, [vec![1], vec![2], vec![127], vec![0x80, 1]]);
}

#[test]
fn should_hold_booleans_and_presences_in_flags() {
    let flags = flags();
    let written = bytes(&flags);

    assert_eq!(written, [0b0010_0111, 0b0001_1011, 1, b't']);
    round_trip(&flags);
    round_trip(&Flags {
        maybe: None,
        nullable: Some(false),
        either: Some(Some(true)),
        text: None,
        ..flags
    });
}

#[test]
fn should_reject_flags_no_field_uses() {
    let result = decode::<Example>(&[0b10, 0, 0]);

    assert_eq!(result, Err(Error::Decode(DecodeFailure::InvalidFlags)));
}

#[test]
fn should_write_characters_as_their_code_points() {
    let characters = CharacterLayout {
        letter: 'é',
        letters: vec![Some('a'), None],
    };
    let written = bytes(&characters);

    assert_eq!(written, [0xe9, 1, 2, 1, b'a', 0]);
    round_trip(&characters);
}

#[test]
fn should_write_a_flattened_struct_where_its_field_is() {
    let printed = printed();
    let written = bytes(&printed);

    assert_eq!(written, [1, b'a', 1]);
    round_trip(&printed);
}

#[test]
fn should_tag_variants_with_their_index() {
    let shapes = shapes();
    let written = shapes.each_ref().map(|shape| bytes(shape)[0]);

    assert_eq!(written, [0, 1, 2]);
    assert_eq!(bytes(&shapes[2]), [2, 1, 2, 3]);
    for shape in &shapes {
        round_trip(shape);
    }
}

#[test]
fn should_reject_an_unknown_variant_or_member() {
    let variant = decode::<Shape>(&[3]);
    let member = decode::<Phase>(&[3]);

    assert_eq!(
        variant,
        Err(Error::Decode(DecodeFailure::UnknownVariant {
            index: 3,
            name: "Shape",
        }))
    );
    assert_eq!(
        member,
        Err(Error::Decode(DecodeFailure::UnknownMember {
            value: 3,
            name: "Phase",
        }))
    );
}

#[test]
fn should_name_fields_after_rust_keywords_raw() {
    let keywords = keywords();
    let written = bytes(&keywords);

    assert_eq!(written, [1, 1, 0]);
    round_trip(&keywords);
}

#[test]
fn should_reject_trailing_and_missing_bytes() {
    let trailing = decode::<Phase>(&[0, 0]);
    let missing = decode::<Example>(&[0, 4, b't']);

    assert_eq!(trailing, Err(Error::Decode(DecodeFailure::InputRemainder)));
    assert_eq!(missing, Err(Error::Decode(DecodeFailure::InputEnd)));
}

#[test]
fn should_reserve_no_more_than_the_input_holds_for_a_list() {
    let result = decode::<Vec<u64>>(&[0xff, 0xff, 0xff, 0xff, 0x0f]);

    assert_eq!(result, Err(Error::Decode(DecodeFailure::InputEnd)));
}

#[test]
fn should_reject_a_varint_beyond_64_bits() {
    let result = decode::<u64>(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02]);

    assert_eq!(result, Err(Error::Decode(DecodeFailure::IntegerOverflow)));
}

#[test]
fn should_round_trip_nested_layouts_with_every_optional_state() {
    for note in [None, Some(None), Some(Some("almost"))] {
        round_trip(&progress(note));
    }
}

#[test]
fn should_write_members_as_their_given_values() {
    let written = bytes(&signals());

    assert_eq!(written, [9, 2, 1, 9]);
    assert_eq!(u32::from(Signal::Kill), 9);
    assert_eq!(Signal::try_from(2), Ok(Signal::Interrupt));
    assert_eq!(Signal::try_from(3), Err(3));
    assert_eq!(
        decode::<Signal>(&[3]),
        Err(Error::Decode(DecodeFailure::UnknownMember {
            value: 3,
            name: "Signal",
        }))
    );
    round_trip(&signals());
}

#[test]
fn should_write_fixed_widths_little_endian_and_bytes_after_their_length() {
    let written = bytes(&raw());

    assert_eq!(
        written,
        [
            0xef, 0xbe, 0xad, 0xde, 5, 0, 0, 0, 0, 1, 0, 0, 3, 1, 2, 3, 2, 0, 1, 255
        ]
    );
    round_trip(&raw());
}

#[test]
fn should_fill_the_defaults_of_a_struct() {
    let defaults = DefaultLayout::default();

    assert_eq!(
        defaults,
        DefaultLayout {
            precision: PRECISION,
            greeting: GREETING.to_owned(),
            signal: Signal::Kill,
            ratio: 0.5,
            offset: -3,
            note: None,
            initial: 'a',
            loud: true,
            level: 7,
        }
    );
}

#[test]
fn should_read_a_field_a_peer_of_an_older_minor_left_out_as_its_default() {
    let changed = DefaultLayout {
        level: 1,
        precision: 4,
        ..DefaultLayout::default()
    };
    let written = bytes_at(&changed, 0);
    let mut reader = BinaryReader::new(&written).with_minor(0);
    let read = DefaultLayout::read(&mut reader)
        .expect("the defaults are decoded from the older minor bytes");

    assert_eq!(
        read,
        DefaultLayout {
            level: 7,
            ..changed
        }
    );
}

#[test]
fn should_match_the_shared_layout_fixtures() {
    let values = layout_values();

    for (name, value) in values {
        Fixtures::check(name, &hex(&value));
    }
}

#[test]
fn should_refuse_a_text_that_is_not_utf8() {
    let bytes = unhex(&Fixtures::get(MALFORMED_TEXT_BODY));
    let read = decode::<Log<'static>>(&bytes);

    assert!(read.is_err());
}

#[test]
fn should_cover_every_shared_fixture() {
    let mut covered: Vec<&str> = layout_values()
        .into_iter()
        .chain(body_values())
        .map(|(name, _)| name)
        .collect();

    covered.extend([
        BATCH,
        ALIASED_BATCH,
        FAILURE_BATCH,
        OPEN_BATCH,
        MALFORMED_TEXT_BODY,
    ]);
    covered.extend(SECURE_FIXTURES);
    covered.sort_unstable();
    assert_eq!(
        covered,
        Fixtures::all()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
}

#[test]
fn should_match_the_shared_body_fixtures() {
    let bodies = body_values();

    for (name, body) in bodies {
        Fixtures::check(name, &hex(&body));
    }
}

#[test]
fn should_match_the_shared_batch_fixture() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();

    user.send(&log("adding"))
        .expect("the log notification is sent");

    let posted = queue.take();

    assert_eq!(posted.len(), 1);
    Fixtures::check(BATCH, &hex(&posted[0]));
}

#[test]
fn should_match_the_shared_aliased_batch_fixture() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();

    user.send(&log("adding"))
        .expect("the log notification is sent");

    let _announced = queue.take();

    user.send(&log("adding"))
        .expect("the log notification is sent");

    let posted = queue.take();

    assert_eq!(posted.len(), 1);
    Fixtures::check(ALIASED_BATCH, &hex(&posted[0]));
}

#[test]
fn should_read_the_alias_a_batch_announced() {
    let (_user, calculator, queue) = Memory::connected_pair::<User>();
    let logged = logs(&calculator);
    let batches = [BATCH, ALIASED_BATCH].map(|name| unhex(&Fixtures::get(name)));

    for batch in batches {
        queue.post(Role::Provider, batch);
    }

    queue.deliver();
    assert_eq!(*logged.borrow(), ["adding", "adding"]);
}

#[test]
fn should_match_the_shared_open_batch_fixture() {
    let (_member, _service, queue) =
        Memory::open_pair::<Member>(ChannelOptions::default(), &credentials());
    let posted = queue.take();

    Fixtures::check(OPEN_BATCH, &hex(&posted[0]));
}

#[test]
fn should_match_the_shared_failure_batch_fixture() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    calculator.handle(|_: Divide, response, _next| response.fail(DivisionFailure::Refusal));

    let _declined = user
        .send(&Divide {
            dividend: 1,
            divisor: 2,
            precision: None,
        })
        .expect("the request is sent");
    let _unhandled = user.send(&Add { a: 1, b: 2 }).expect("the request is sent");

    queue.step();
    calculator.close(ErrorCode::ClosedChannel);

    let posted = queue.take_batches();

    assert_eq!(posted.len(), 1);
    Fixtures::check(FAILURE_BATCH, &hex(&posted[0]));
}

#[test]
fn should_read_the_failures_of_the_shared_failure_batch_fixture() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();
    let declined = user
        .send(&Divide {
            dividend: 1,
            divisor: 2,
            precision: None,
        })
        .expect("the request is sent");
    let unhandled = user.send(&Add { a: 1, b: 2 }).expect("the request is sent");
    let _ = queue.take();
    let fixture = unhex(&Fixtures::get(FAILURE_BATCH));

    queue.post(Role::Client, fixture);
    queue.deliver();
    assert_eq!(
        ready_output(declined),
        Err(Error::Failure(DivisionFailure::Refusal))
    );
    assert_eq!(
        ready_output(unhandled),
        Err(Error::Rejection(ErrorCode::UnhandledMessage))
    );
    assert_eq!(user.close_reason(), Some(ErrorCode::ClosedChannel));
}

#[test]
fn should_take_a_frame_length_of_several_bytes() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();
    let text = "x".repeat(300);

    user.send(&log(&text))
        .expect("the log notification is sent");

    let batch = Slice::decode(&queue.take()[0]);

    assert_eq!(
        batch.frames,
        [Frame::Message {
            type_id: 0,
            id: None,
            body: bytes(&log(&text)),
        }]
    );
}

#[test]
fn should_receive_the_frames_of_a_batch_in_order() {
    let (user, calculator, queue) = Memory::pair::<User>();
    let logged = logs(&calculator);

    for text in ["first", "second"] {
        user.send(&log(text)).expect("the log notification is sent");
    }

    queue.deliver();
    assert_eq!(*logged.borrow(), ["first", "second"]);
}

#[test]
fn should_borrow_the_strings_of_a_message_body() {
    let text = String::from("borrowed");
    let log = Log {
        text: text.as_str().into(),
    };

    assert!(matches!(log.text, std::borrow::Cow::Borrowed(_)));
    assert_eq!(bytes(&log), bytes(&self::log("borrowed")));
}

#[test]
fn should_write_into_a_reused_writer() {
    let mut writer = BinaryWriter::new();

    log("first")
        .write(&mut writer)
        .expect("the log body is encoded");
    writer.clear();
    log("second")
        .write(&mut writer)
        .expect("the log body is encoded");
    assert_eq!(writer.as_slice(), bytes(&log("second")));
}

#[test]
fn should_validate_a_value_read_borrowing_its_bytes_as_its_owned_form() {
    let written = bytes(&Step {
        name: "Not Validated".to_owned(),
        result: None,
        duration: None,
    });
    let reader = || BinaryReader::new(&written).with_validation(true);
    let borrowed = StepRef::read_ref(&mut reader()).map(StepRef::into_owned);
    let owned = Step::read_with_validation(&mut reader(), &Validation::Any);

    assert!(matches!(borrowed, Err(Error::Invalid(_))));
    assert_eq!(borrowed, owned);
}

#[test]
fn should_read_a_value_borrowing_its_bytes_as_its_owned_form() {
    let written = bytes(&progress(Some(Some("almost"))));
    let borrowed = ProgressRef::read_ref(&mut BinaryReader::new(&written))
        .expect("the progress is read")
        .into_owned();

    assert_eq!(borrowed, progress(Some(Some("almost"))));
}
