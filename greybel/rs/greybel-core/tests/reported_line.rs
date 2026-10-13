//! Tests that parse errors name the line Grey Hack's parser reports them at, as measured in the game.

use crate::support::reported_line::reported_line;

#[test]
fn should_report_an_error_without_a_line_of_its_own_at_its_statement() {
    let sources = [
        "x = {1:}",
        "for i in\nend for",
        "print \"abc",
        "else",
        "x = 1\nwhile true\nend for",
    ];

    let lines = sources.map(reported_line);

    assert_eq!(lines, [1, 1, 1, 1, 3]);
}

#[test]
fn should_report_an_expected_token_where_the_lexer_is_after_it() {
    let sources = ["x = (1", "x = 1 2", "f = function(a\nend function"];

    let lines = sources.map(reported_line);

    assert_eq!(lines, [1, 1, 2]);
}

#[test]
fn should_report_an_unclosed_block_after_the_last_line() {
    let sources = ["if true then\nprint 1", "break"];

    let lines = sources.map(reported_line);

    assert_eq!(lines, [3, 2]);
}

/// Grey Hack 0.9.7373 reports a block left open in a function at its `end function` (`Parser.CheckForOpenBackpatches`).
#[test]
fn should_report_a_block_left_open_in_a_function_at_its_end() {
    let sources = [
        "f = function()\nif true then\nprint 1\nend function\nprint 2",
        "f = function()\nfor i in [1]\nend function",
        "f = function()\nwhile 1\nend function\nx = 1",
    ];

    let lines = sources.map(reported_line);

    assert_eq!(lines, [4, 3, 3]);
}
