//! Layout on a terminal cell grid: `<pos>`, `<space>` and `<voffset>` place characters in columns and rows, overlapping as in the game.

use crate::support::layout::render_in_game_cells;
use crate::support::render;

#[test]
fn should_draw_a_prompt_built_from_positions_offsets_and_rotated_dashes() {
    let text = "<pos=4.95>———(root@mainframe)<voffset=-0.35em><pos=0><rotate=90>—<pos=0><voffset=-0.7em>—<pos=0><voffset=-1.05em>—<rotate=0><pos=4.95><voffset=-1.4em>——$ ";
    let shown = render_in_game_cells(text);

    assert_eq!(shown, "│———(root@mainframe)\n│——$ ");
}

#[test]
fn should_draw_over_earlier_text_after_moving_back() {
    let text = "abcd<pos=0>XY";
    let shown = render(text);

    assert_eq!(shown, "XYcd");
}

#[test]
fn should_not_erase_earlier_text_with_a_space() {
    let text = "abcd<pos=0> X";
    let shown = render(text);

    assert_eq!(shown, "aXcd");
}

#[test]
fn should_erase_earlier_text_with_a_marked_space() {
    let text = "ab<pos=0><mark=#ff0000> </mark>";
    let shown = render(text);

    assert_eq!(shown, "\x1b[48;2;255;0;0m \x1b[0mb");
}

#[test]
fn should_leave_a_gap_after_moving_forward() {
    let text = "a<pos=3em>b";
    let shown = render(text);

    assert_eq!(shown, "a    b");
}

#[test]
fn should_add_a_space_in_cells() {
    let text = "a<space=1.2em>b";
    let shown = render(text);

    assert_eq!(shown, "a  b");
}

#[test]
fn should_ignore_a_position_in_percent_without_a_width() {
    let text = "ab<pos=50%>c";
    let shown = render(text);

    assert_eq!(shown, "abc");
}

#[test]
fn should_put_raised_text_on_a_row_above() {
    let text = "a<voffset=1em>b";
    let shown = render(text);

    assert_eq!(shown, " b\na");
}

#[test]
fn should_keep_a_small_offset_on_its_row() {
    let text = "a<voffset=6>b";
    let shown = render(text);

    assert_eq!(shown, "ab");
}

#[test]
fn should_start_a_line_break_below_the_lowest_offset_text() {
    let text = "a<voffset=-1.25em>b</voffset><br>c";
    let shown = render(text);

    assert_eq!(shown, "a\n b\nc");
}

#[test]
fn should_turn_dashes_and_bars_a_quarter_turn() {
    let text = "<rotate=90>—-|</rotate><rotate=-90>━</rotate><rotate=45>—</rotate><rotate=180>—";
    let shown = render(text);

    assert_eq!(shown, "││─┃——");
}

#[test]
fn should_draw_a_sprite_as_a_block_in_its_color_without_advancing() {
    let text = "a<sprite index=0 color=#ff0000>";
    let shown = render(text);

    assert_eq!(shown, "a\x1b[38;2;255;0;0m█\x1b[0m");
}
