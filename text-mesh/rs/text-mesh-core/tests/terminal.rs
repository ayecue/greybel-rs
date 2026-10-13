//! The game's terminal splitting its output into lines.

use text_mesh_core::TerminalOutput;

#[test]
fn should_split_at_every_line_break() {
    let output = TerminalOutput::new("a\r\nb\rc\nd\n");
    let lines = output.lines();

    assert_eq!(lines, ["a", "b", "c", "d", ""]);
}

#[test]
fn should_cut_a_long_line_into_pieces_of_180() {
    let text = "x".repeat(300);
    let lengths: Vec<usize> = TerminalOutput::new(&text)
        .lines()
        .iter()
        .map(String::len)
        .collect();

    assert_eq!(lengths, [180, 120]);
}

#[test]
fn should_keep_a_long_line_whose_text_without_tags_is_short() {
    let text = "<b>a</b>".repeat(40);
    let lines = TerminalOutput::new(&text).lines();

    assert_eq!(lines, [text]);
}

#[test]
fn should_cut_a_line_mostly_of_tags_into_pieces_of_900() {
    let text = "<b>a".repeat(270);
    let lengths: Vec<usize> = TerminalOutput::new(&text)
        .lines()
        .iter()
        .map(String::len)
        .collect();

    assert_eq!(lengths, [900, 180]);
}

#[test]
fn should_cut_a_line_with_very_many_tags_into_pieces_of_180() {
    let text = "<b>x</b>".repeat(300);
    let first = TerminalOutput::new(&text).lines()[0].len();

    assert_eq!(first, 180);
}

#[test]
fn should_count_line_lengths_in_utf16_units() {
    let text = "é".repeat(200);
    let lines = TerminalOutput::new(&text).lines();

    assert_eq!(lines, [text]);
}
