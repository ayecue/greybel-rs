//! Layout tags as CSS: `<pos>` overlaps segments in a cell, `<voffset>` shifts the baseline and `<rotate>`/`<scale>` transform characters.

use crate::support::render;

#[test]
fn should_stack_the_segments_of_positions_in_one_grid_cell() {
    let text = "ab<pos=10>c";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"display:grid;align-items:baseline\">\
         <span style=\"grid-area:1/1;justify-self:start;margin-left:0\">ab</span>\
         <span style=\"grid-area:1/1;justify-self:start;margin-left:0.5em\">c</span>\
         </span>"
    );
}

#[test]
fn should_place_a_position_in_percent_of_the_line() {
    let text = "<pos=50%>a";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"display:grid;align-items:baseline\">\
         <span style=\"grid-area:1/1;justify-self:start;margin-left:50%\">a</span>\
         </span>"
    );
}

#[test]
fn should_start_the_line_after_a_positioned_line_without_another_break() {
    let text = "a<pos=10>b<br>c";
    let shown = render(text);

    assert!(shown.ends_with("</span>c"));
}

#[test]
fn should_add_a_space_as_an_empty_block() {
    let text = "a<space=1em>b";
    let shown = render(text);

    assert_eq!(
        shown,
        "a<span style=\"display:inline-block;width:1em\"></span>b"
    );
}

#[test]
fn should_shift_the_baseline_in_ems_of_the_texts_own_size() {
    let text = "<size=10><voffset=-5>a";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"font-size:0.5em;vertical-align:-0.5em\">a</span>"
    );
}

#[test]
fn should_rotate_each_character_counterclockwise() {
    let text = "<rotate=90>ab";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"display:inline-block;transform:rotate(-90deg)\">a</span>\
         <span style=\"display:inline-block;transform:rotate(-90deg)\">b</span>"
    );
}

#[test]
fn should_scale_each_character_horizontally() {
    let text = "<scale=1.5>a";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"display:inline-block;width:1.5ch\">\
         <span style=\"display:inline-block;transform:scaleX(1.5);transform-origin:left\">a</span>\
         </span>"
    );
}

#[test]
fn should_size_and_offset_a_sprite_with_its_text() {
    let text = "<size=40><voffset=-10><sprite=0>";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span class=\"sprite\" data-index=\"0\" style=\"color:rgb(255,255,255);font-size:2em;vertical-align:-0.25em\"></span>"
    );
}
