//! Tests rich-text tags rendered as ANSI with terminal-column layout.

use crate::support::Harness;

#[test]
fn should_retain_text_in_the_terminal_colour_without_escape_sequences() {
    let mut harness = Harness::new();

    let texts = harness.render(&[("hello", 0)]);

    assert_eq!(texts, ["hello"]);
}

#[test]
fn should_render_colours_and_styles_as_escape_sequences() {
    let mut harness = Harness::new();

    let texts = harness.render(&[("<color=#FF0000>red</color> and <b>bold</b>", 0)]);

    assert_eq!(
        texts,
        ["\x1b[38;2;255;0;0mred\x1b[0m and \x1b[1mbold\x1b[0m"]
    );
}

#[test]
fn should_read_colour_names_as_the_game_does() {
    let mut harness = Harness::new();

    let texts = harness.render(&[("<color=yellow>yellow</color>", 0)]);

    assert_eq!(texts, ["\x1b[38;2;255;235;4myellow\x1b[0m"]);
}

#[test]
fn should_wrap_lines_at_the_columns() {
    let mut harness = Harness::new();

    let texts = harness.render(&[("one two three four five six seven", 12)]);

    assert_eq!(texts, ["one two\nthree four\nfive six\nseven"]);
}

#[test]
fn should_respond_with_every_text_in_order() {
    let mut harness = Harness::new();

    let texts = harness.render(&[("first", 0), ("<i>second</i>", 0), ("third", 40)]);

    assert_eq!(texts, ["first", "\x1b[3msecond\x1b[0m", "third"]);
}
