//! Tests stripping rich-text markup to visible terminal characters.

use greyhack_terminal_text::plain_text;

#[test]
fn should_remove_tags() {
    assert_eq!(
        plain_text("<color=#FF0000>red</color> and <b>bold</b>"),
        "red and bold"
    );
}

#[test]
fn should_retain_lines_and_text_that_is_not_a_tag() {
    assert_eq!(plain_text("a < b\n<i>c</i>"), "a < b\nc");
}
