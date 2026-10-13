//! Style tags and the stacks they push onto.

use text_mesh_core::{Color, FontWeight};

use crate::support::{BASE_SIZE, plain, style_of, styles};

#[test]
fn should_make_text_bold_until_closed() {
    let text = "<b>a</b>b";
    let shown = [style_of(text, "a"), style_of(text, "b")].map(|style| (style.bold, style.weight));

    assert_eq!(
        shown,
        [(true, FontWeight::BOLD), (false, FontWeight::REGULAR)]
    );
}

#[test]
fn should_count_nested_bold_tags() {
    let text = "<b><b>a</b>b</b>c";
    let shown = [style_of(text, "b").bold, style_of(text, "c").bold];

    assert_eq!(shown, [true, false]);
}

#[test]
fn should_set_a_font_weight() {
    let text = "<font-weight=300>x";
    let shown = style_of(text, "x").weight;

    assert_eq!(shown, FontWeight::LIGHT);
}

#[test]
fn should_cap_the_size_at_60() {
    let text = "<size=100>x";
    let shown = style_of(text, "x").size;

    assert_eq!(shown, 60.0);
}

#[test]
fn should_read_relative_and_unit_sizes() {
    let texts = ["<size=+5>x", "<size=-5>x", "<size=50%>x", "<size=1.5em>x"];
    let sizes = texts.map(|text| style_of(text, "x").size);

    assert_eq!(
        sizes,
        [
            BASE_SIZE + 5.0,
            BASE_SIZE - 5.0,
            BASE_SIZE / 2.0,
            BASE_SIZE * 1.5
        ]
    );
}

#[test]
fn should_read_a_size_without_digits_as_zero() {
    let text = "<size=abc>x";
    let shown = style_of(text, "x").size;

    assert_eq!(shown, 0.0);
}

#[test]
fn should_go_back_to_the_previous_size_on_closing() {
    let text = "<size=30><size=40>a</size>b";
    let shown = style_of(text, "b").size;

    assert_eq!(shown, 30.0);
}

#[test]
fn should_change_the_case_of_letters() {
    let text = "<uppercase>abß</uppercase> <lowercase>CD</lowercase> <allcaps>e</allcaps>";
    let shown = plain(text);

    assert_eq!(shown, "ABß cd E");
}

#[test]
fn should_shrink_only_the_lowercase_letters_of_small_caps() {
    let text = "<smallcaps>aB";
    let shown: Vec<(String, bool)> = styles(text)
        .into_iter()
        .map(|(run, style)| (run, style.small_caps))
        .collect();

    assert_eq!(shown, vec![("A".to_owned(), true), ("B".to_owned(), false)]);
}

#[test]
fn should_prefer_uppercase_over_small_caps() {
    let text = "<smallcaps><uppercase>a";
    let shown = style_of(text, "A").small_caps;

    assert!(!shown);
}

#[test]
fn should_end_small_caps_on_the_first_closing() {
    let text = "<smallcaps><smallcaps>a</smallcaps>b";
    let shown = plain(text);

    assert_eq!(shown, "Ab");
}

#[test]
fn should_pop_the_underline_color_twice_on_closing() {
    let text = "<u color=#ff0000><u color=#00ff00>a</u>b";
    let shown = style_of(text, "b").underline;

    assert_eq!(shown, Some(Color::WHITE));
}

#[test]
fn should_color_a_strikethrough() {
    let text = "<s color=#ff0000>x";
    let shown = style_of(text, "x").strikethrough;

    assert_eq!(shown, Some(Color::new(255, 0, 0, 255)));
}

#[test]
fn should_highlight_in_translucent_yellow_by_default() {
    let text = "<mark>x";
    let shown = style_of(text, "x").highlight;

    assert_eq!(shown, Some(Color::new(255, 255, 0, 64)));
}

#[test]
fn should_read_the_highlight_color_from_mark_or_its_lowercase_color() {
    let texts = [
        "<mark=#ff000080>x",
        "<mark color=#00ff00>x",
        "<mark COLOR=#00ff00>x",
    ];
    let shown = texts.map(|text| style_of(text, "x").highlight);

    assert_eq!(
        shown,
        [
            Some(Color::new(255, 0, 0, 128)),
            Some(Color::new(0, 255, 0, 255)),
            Some(Color::new(255, 255, 0, 64))
        ]
    );
}

#[test]
fn should_italicize_even_when_the_angle_rejects_the_tag() {
    let text = "<i angle=200>x";
    let shown = (plain(text), style_of(text, "x").italic);

    assert_eq!(shown, ("<i angle=200>x".to_owned(), true));
}

#[test]
fn should_mark_subscript_and_superscript() {
    let text = "<sub>a</sub><sup>b</sup>";
    let shown = [style_of(text, "a"), style_of(text, "b")]
        .map(|style| (style.subscript, style.superscript));

    assert_eq!(shown, [(true, false), (false, true)]);
}

#[test]
fn should_read_spacing_and_offsets() {
    let text = "<cspace=2><mspace=1em><voffset=3>x";
    let style = style_of(text, "x");

    assert_eq!(
        (
            style.character_spacing,
            style.mono_spacing,
            style.vertical_offset
        ),
        (2.0, BASE_SIZE, 3.0)
    );
}

#[test]
fn should_show_a_percentage_where_the_game_rejects_it_as_text() {
    let text = "<voffset=5%><cspace=5%><space=5%>x";
    let shown = plain(text);

    assert_eq!(shown, text);
}
