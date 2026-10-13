//! Tests CSS for indentation, no-break, and fixed-width line layout tags.

use crate::support::render;

#[test]
fn should_put_a_line_with_margins_in_a_padded_block() {
    let text = "a<margin=10%>b";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"display:block;padding-left:10%;padding-right:10%\">ab</span>"
    );
}

#[test]
fn should_keep_margins_on_later_lines_while_they_last() {
    let text = "<margin=20>a<br>b</margin><br>c";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"display:block;padding-left:1em;padding-right:1em\">a</span>\
         <span style=\"display:block;padding-left:1em;padding-right:1em\">b</span>c"
    );
}

#[test]
fn should_keep_a_margin_that_ends_after_the_last_visible_character() {
    let text = "a<margin=10%>b</margin>";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"display:block;padding-left:10%;padding-right:10%\">ab</span>"
    );
}

#[test]
fn should_drop_a_margin_that_starts_after_the_last_visible_character() {
    let text = "a <margin=10%> ";
    let shown = render(text);

    assert_eq!(shown, "a  ");
}

#[test]
fn should_align_a_line_with_its_last_alignment() {
    let text = "<align=right>a<align=center>b";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"display:block;text-align:center\">ab</span>"
    );
}

#[test]
fn should_justify_the_last_line_of_a_flush_line_too() {
    let text = "<align=flush>a b";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"display:block;text-align:justify;text-align-last:justify\">a b</span>"
    );
}

#[test]
fn should_limit_the_width_of_a_line() {
    let text = "<width=50%>a";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"display:block;max-width:50%\">a</span>"
    );
}

#[test]
fn should_hang_wrapped_text_at_an_indent() {
    let text = "a<indent=40>b";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"display:grid;align-items:baseline\">\
         <span style=\"grid-area:1/1;justify-self:start;margin-left:0\">a</span>\
         <span style=\"grid-area:1/1;justify-self:start;margin-left:2em\">b</span>\
         </span>"
    );
}

#[test]
fn should_indent_the_tag_and_the_start_of_later_lines() {
    let text = "a<line-indent=20>b<br>c";
    let shown = render(text);

    assert_eq!(
        shown,
        "a<span style=\"display:inline-block;width:1em\"></span>b<br>\
         <span style=\"display:inline-block;width:1em\"></span>c"
    );
}

#[test]
fn should_keep_text_without_breaks_on_one_line() {
    let text = "a <nobr>b c</nobr> d";
    let shown = render(text);

    assert_eq!(shown, "a <span style=\"white-space:nowrap\">b c</span> d");
}

#[test]
fn should_center_each_character_in_its_monospace_width() {
    let text = "<mspace=30>ab";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"display:inline-block;width:1.5em;text-align:center\"><span style=\"display:inline\">a</span></span>\
         <span style=\"display:inline-block;width:1.5em;text-align:center\"><span style=\"display:inline\">b</span></span>"
    );
}
