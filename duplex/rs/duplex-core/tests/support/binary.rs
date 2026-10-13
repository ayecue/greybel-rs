use std::collections::BTreeMap;
use std::fmt::Debug;

use duplex_core::{BinaryDecode, BinaryEncode, Bytes, Fixed32, Fixed64};
use duplex_test_support::binary::{bytes, bytes_at, decode};
use duplex_test_support::conformance::calculator::{Phase, Progress, Quotient, Step};
use duplex_test_support::conformance::layouts::{
    CharacterLayout, DefaultLayout, Example, Flags, Keyword, KeywordLayout, NumberLayout, Options,
    PrintLayout, Raw, Shape, Signal, SignalLayout,
};

use crate::support::log;

pub const BATCH: &str = "notificationBatch";

pub const ALIASED_BATCH: &str = "aliasedBatch";

pub const FAILURE_BATCH: &str = "failureBatch";

pub const OPEN_BATCH: &str = "openBatch";

pub const MALFORMED_TEXT_BODY: &str = "malformedTextBody";

pub fn round_trip<T: BinaryEncode + BinaryDecode + PartialEq + Debug>(value: &T) {
    assert_eq!(decode::<T>(&bytes(value)).as_ref(), Ok(value));
}

#[expect(clippy::option_option, reason = "the note is optional and nullable")]
pub fn progress(note: Option<Option<&str>>) -> Progress<'static> {
    Progress {
        phase: Phase::Active,
        done_share: Some(0.75),
        steps: vec![
            Step {
                name: "exact".to_owned(),
                result: Some(Quotient::Exact { value: 2 }),
                duration: None,
            },
            Step {
                name: "remainder".to_owned(),
                result: Some(Quotient::WithRemainder {
                    value: 3,
                    remainder: 1,
                }),
                duration: None,
            },
            Step {
                name: "waiting".to_owned(),
                result: None,
                duration: None,
            },
        ]
        .into(),
        counts: BTreeMap::from([("done".to_owned(), 2), ("total".to_owned(), 3)]),
        note: note.map(|note| note.map(|note| note.to_owned().into())),
        eta: None,
    }
}

pub fn layout_values() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("defaults", bytes(&DefaultLayout::default())),
        ("defaultsOfMinor0", bytes_at(&DefaultLayout::default(), 0)),
        ("example", bytes(&example())),
        (
            "numbers",
            bytes(&NumberLayout {
                u8: 255,
                u16: 65_535,
                u32: 4_000_000_000,
                u64: 9_007_199_254_740_991,
                i8: -128,
                i16: -32_768,
                i32: -2_147_483_648,
                i64: -9_007_199_254_740_991,
                f32: -1.25,
                f64: 0.1,
            }),
        ),
        ("flags", bytes(&flags())),
        ("otherFlags", bytes(&other_flags())),
        (
            "characters",
            bytes(&CharacterLayout {
                letter: 'é',
                letters: vec![Some('a'), None, Some('😀')],
            }),
        ),
        ("printed", bytes(&printed())),
        ("point", bytes(&shapes()[0])),
        ("circle", bytes(&shapes()[1])),
        ("rectangle", bytes(&shapes()[2])),
        ("keywords", bytes(&keywords())),
        ("raw", bytes(&raw())),
        ("signals", bytes(&signals())),
    ]
}

pub fn example() -> Example {
    Example {
        text: "test".to_owned(),
        count: 300,
        flag: true,
    }
}

pub fn flags() -> Flags {
    Flags {
        first: true,
        maybe: Some(true),
        nullable: None,
        either: Some(None),
        text: Some(Some("t".to_owned())),
        second: false,
        third: true,
        fourth: true,
    }
}

/// Returns [`flags`] with every flag toggled to its opposite state.
pub fn other_flags() -> Flags {
    Flags {
        first: false,
        maybe: None,
        nullable: Some(false),
        either: Some(Some(true)),
        text: None,
        second: true,
        third: false,
        fourth: false,
    }
}

pub fn printed() -> PrintLayout {
    PrintLayout {
        text: "a".to_owned(),
        options: Options {
            new_line: true,
            replace: false,
        },
    }
}

/// Returns one `Shape` for each variant, in declaration order.
pub fn shapes() -> [Shape; 3] {
    [
        Shape::Point,
        Shape::Circle(0.5),
        Shape::Rectangle {
            width: 2,
            height: 3,
            filled: true,
        },
    ]
}

pub fn keywords() -> KeywordLayout {
    KeywordLayout {
        r#type: Keyword::Match,
        r#match: vec![Keyword::Type],
    }
}

pub fn raw() -> Raw {
    Raw {
        hash: Fixed32(0xdead_beef),
        seed: Fixed64((1 << 40) + 5),
        data: Bytes(vec![1, 2, 3]),
        chunks: vec![Bytes(Vec::new()), Bytes(vec![255])],
    }
}

pub fn signals() -> SignalLayout {
    SignalLayout {
        first: Signal::Kill,
        all: vec![Signal::Hang, Signal::Kill],
    }
}

pub fn body_values() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("logBody", bytes(&log("adding"))),
        ("progressBody", bytes(&progress(Some(Some("almost"))))),
        ("progressAbsentBody", bytes(&progress(None))),
        ("progressNullBody", bytes(&progress(Some(None)))),
    ]
}
