//! Which tags TMP accepts; a rejected tag shows as text.

use crate::support::{plain, style_of};

#[test]
fn should_hide_an_accepted_tag() {
    let text = "<b>bold</b>";
    let shown = plain(text);

    assert_eq!(shown, "bold");
}

#[test]
fn should_show_an_unknown_tag_as_text() {
    let text = "<foo>x";
    let shown = plain(text);

    assert_eq!(shown, "<foo>x");
}

#[test]
fn should_show_a_mixed_case_tag_as_text() {
    let text = "<Color=red>x";
    let shown = plain(text);

    assert_eq!(shown, "<Color=red>x");
}

#[test]
fn should_accept_an_uppercase_tag() {
    let text = "<COLOR=red>x";
    let shown = plain(text);

    assert_eq!(shown, "x");
}

#[test]
fn should_accept_one_attribute_after_the_name() {
    let text = "<b c>x";
    let shown = plain(text);

    assert_eq!(shown, "x");
}

#[test]
fn should_show_a_tag_with_a_second_space_in_a_name_as_text() {
    let text = "<b c d>x";
    let shown = plain(text);

    assert_eq!(shown, "<b c d>x");
}

#[test]
fn should_accept_a_tag_of_127_characters_between_its_brackets() {
    let text = format!("<link={}>x", "a".repeat(122));
    let shown = plain(&text);

    assert_eq!(shown, "x");
}

#[test]
fn should_show_a_tag_of_128_characters_between_its_brackets_as_text() {
    let text = format!("<link={}>x", "a".repeat(123));
    let shown = plain(&text);

    assert_eq!(shown, text);
}

#[test]
fn should_show_an_opening_bracket_inside_a_tag_as_text() {
    let text = "<b<i>x";
    let shown = plain(text);

    assert_eq!(shown, "<bx");
}

#[test]
fn should_show_tags_inside_noparse_as_text() {
    let text = "<noparse><b>x</b></noparse>y";
    let shown = plain(text);

    assert_eq!(shown, "<b>x</b>y");
}

#[test]
fn should_not_style_text_inside_noparse() {
    let text = "<noparse><b>x</b></noparse>";
    let style = style_of(text, "x");

    assert!(!style.bold);
}

#[test]
fn should_hide_a_closing_noparse_outside_noparse() {
    let text = "a</noparse>b";
    let shown = plain(text);

    assert_eq!(shown, "ab");
}

#[test]
fn should_show_closing_tags_the_game_has_no_case_for_as_text() {
    let text = "</alpha></space></page></sprite></margin-left>";
    let shown = plain(text);

    assert_eq!(shown, text);
}

#[test]
fn should_hide_a_lowercase_closing_a_only() {
    let text = "</a></A>";
    let shown = plain(text);

    assert_eq!(shown, "</A>");
}

#[test]
fn should_hide_table_rows_and_headers_but_not_cells() {
    let text = "<tr><th></th></tr></table><td><table>";
    let shown = plain(text);

    assert_eq!(shown, "<td><table>");
}

#[test]
fn should_show_a_tag_without_its_required_value_as_text() {
    let text = "<size>x";
    let shown = plain(text);

    assert_eq!(shown, "<size>x");
}

#[test]
fn should_show_a_tag_with_more_than_eight_attributes_as_text() {
    let text = "<b a=1 b=2 c=3 d=4 e=5 f=6 g=7 h=8>x";
    let shown = plain(text);

    assert_eq!(shown, text);
}

#[test]
fn should_accept_only_fonts_of_the_assets() {
    let text = "<font=default>a</font><font=\"liberationsans sdf\">b</font><font=\"Roboto\">c";
    let shown = plain(text);

    assert_eq!(shown, "ab<font=\"Roboto\">c");
}

#[test]
fn should_accept_a_font_with_a_material_of_the_assets() {
    let text = "<font=\"LiberationSans SDF\" material=\"LiberationSans SDF - Outline\">a<font=\"LiberationSans SDF\" material=\"Glow\">b";
    let shown = plain(text);

    assert_eq!(shown, "a<font=\"LiberationSans SDF\" material=\"Glow\">b");
}

#[test]
fn should_accept_only_materials_of_the_assets() {
    let text = "<material=\"SourceCodePro-Regular SDF\">a<material=\"LiberationSans SDF\">b";
    let shown = plain(text);

    assert_eq!(shown, "a<material=\"LiberationSans SDF\">b");
}

#[test]
fn should_accept_only_known_alignments() {
    let text = "<align=center>a</align><align=Center>b";
    let shown = plain(text);

    assert_eq!(shown, "a<align=Center>b");
}
