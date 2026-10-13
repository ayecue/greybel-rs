use std::borrow::Cow;

use duplex_test_support::binary::{bytes, decode};
use greybel_io::{Echo, Key, Output};
use greybel_io_web::protocol::{OutputBatch, ReadKey, ReadLine};
use greybel_test_support::io::print;

use crate::support::{fixture, keys};

#[test]
fn should_lay_out_the_messages_as_the_fixture() {
    let entries = [
        print("10%", true, false),
        print("20%", true, true),
        Output::Clear,
        print("name: ", false, false),
    ];
    let output = OutputBatch {
        entries: Cow::Borrowed(&entries),
    };
    let read_line = ReadLine {
        prompt: Cow::Borrowed("password: "),
        echo: Echo::Masked,
    };
    let read_key = ReadKey {
        prompt: Cow::Borrowed(""),
    };
    let laid_out = [bytes(&output), bytes(&read_line), bytes(&read_key)];

    assert_eq!(
        laid_out,
        [
            fixture(&["output"]),
            fixture(&["readLine"]),
            fixture(&["readKey"])
        ]
    );
}

#[test]
fn should_read_the_fixture_line_reply() {
    let line = decode::<String>(&fixture(&["line"]));

    assert_eq!(line, Ok("hunter2".to_owned()));
}

#[test]
fn should_read_the_fixture_key_replies() {
    let cases = keys();
    let read: Vec<_> = cases
        .iter()
        .map(|(case, _)| decode::<Key>(&fixture(&["keys", case])))
        .collect();

    for ((case, key), read) in cases.into_iter().zip(read) {
        assert_eq!(read, Ok(key), "{case}");
    }
}

#[test]
fn should_lay_out_keys_as_the_fixture_replies() {
    let cases = keys();
    let sent: Vec<_> = cases.iter().map(|(_, key)| bytes(key)).collect();

    for ((case, _), sent) in cases.into_iter().zip(sent) {
        assert_eq!(sent, fixture(&["keys", case]), "{case}");
    }
}
