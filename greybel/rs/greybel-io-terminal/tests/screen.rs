use greybel_io::Output;
use greybel_test_support::io::print;

use crate::support::screen::{CLEAR, ERASE_LINE, PREVIOUS_LINE, screen_text};

#[test]
fn should_move_up_to_a_full_line_when_replacing_after_it() {
    let batch = [print("10%", true, false), print("20%", true, true)];
    let out = screen_text(&batch);

    assert_eq!(out, format!("10%\n{PREVIOUS_LINE}{ERASE_LINE}20%\n"));
}

#[test]
fn should_erase_an_open_line_in_place_when_replacing_it() {
    let batch = [print("loading", false, false), print("done", true, true)];
    let out = screen_text(&batch);

    assert_eq!(out, format!("loading{ERASE_LINE}done\n"));
}

#[test]
fn should_only_print_when_replacing_on_a_blank_screen() {
    let batch = [
        print("a", true, false),
        Output::Clear,
        print("b", true, true),
    ];
    let out = screen_text(&batch);

    assert_eq!(out, format!("a\n{CLEAR}b\n"));
}
