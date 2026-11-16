//! Color tags: hex, named and alpha forms.

use text_mesh_core::Color;

use crate::support::{component_in, plain, style_in, style_of};

#[test]
fn should_read_the_short_hex_forms() {
    let texts = ["<#f00>x", "<#f008>x"];
    let colors = texts.map(|text| style_of(text, "x").color);

    assert_eq!(
        colors,
        [Color::new(255, 0, 0, 255), Color::new(255, 0, 0, 0x88)]
    );
}

#[test]
fn should_read_the_long_hex_forms() {
    let texts = ["<#ff8000>x", "<#ff800080>x"];
    let colors = texts.map(|text| style_of(text, "x").color);

    assert_eq!(
        colors,
        [Color::new(255, 128, 0, 255), Color::new(255, 128, 0, 128)]
    );
}

#[test]
fn should_read_the_hex_forms_of_color() {
    let texts = [
        "<color=#0f0>x",
        "<color=#0f08>x",
        "<color=#00ff00>x",
        "<color=#00ff0080>x",
    ];
    let colors = texts.map(|text| style_of(text, "x").color);

    assert_eq!(
        colors,
        [
            Color::new(0, 255, 0, 255),
            Color::new(0, 255, 0, 0x88),
            Color::new(0, 255, 0, 255),
            Color::new(0, 255, 0, 128)
        ]
    );
}

#[test]
fn should_read_any_character_that_is_not_a_hex_digit_as_f() {
    let text = "<#zz0>x";
    let color_shown = style_of(text, "x").color;

    assert_eq!(color_shown, Color::new(255, 255, 0, 255));
}

#[test]
fn should_show_a_quoted_hex_color_as_text() {
    let text = "<color=\"#f00\">x";
    let shown = plain(text);

    assert_eq!(shown, text);
}

#[test]
fn should_read_the_named_colors() {
    let texts = ["<color=yellow>x", "<color=grey>x", "<color=lightblue>x"];
    let colors = texts.map(|text| style_of(text, "x").color);

    assert_eq!(
        colors,
        [
            Color::new(255, 235, 4, 255),
            Color::new(128, 128, 128, 255),
            Color::new(173, 216, 230, 255)
        ]
    );
}

#[test]
fn should_show_names_the_game_does_not_know_as_text() {
    let text = "<color=gray>a<color=RED>b<color=cyan>c";
    let shown = plain(text);

    assert_eq!(shown, text);
}

#[test]
fn should_go_back_to_the_previous_color_on_closing() {
    let text = "<color=red><color=blue>a</color>b";
    let shown = style_of(text, "b").color;

    assert_eq!(shown, Color::new(255, 0, 0, 255));
}

#[test]
fn should_go_back_to_the_component_color_on_too_many_closings() {
    let text = "<color=red>a</color></color></color>b";
    let shown = style_of(text, "b").color;

    assert_eq!(shown, Color::WHITE);
}

#[test]
fn should_keep_fifteen_colors_on_the_stack() {
    let opened: String = (1..=17)
        .map(|level| format!("<#0000{level:02x}>"))
        .collect();
    let text = format!("{opened}a</color>b");
    let shown = style_of(&text, "b").color;

    assert_eq!(shown, Color::new(0, 0, 14, 255));
}

#[test]
fn should_set_the_alpha_from_fixed_positions() {
    let text = "<alpha=#80>x";
    let shown = style_of(text, "x").color;

    assert_eq!(shown, Color::new(255, 255, 255, 128));
}

#[test]
fn should_show_an_alpha_without_two_hex_digits_as_text() {
    let text = "<alpha=#8>x";
    let shown = plain(text);

    assert_eq!(shown, text);
}

#[test]
fn should_cap_the_alpha_at_the_component_alpha() {
    let component = component_in(Color::new(255, 255, 255, 64));
    let shown = style_in(&component, "<color=red>x", "x").color;

    assert_eq!(shown, Color::new(255, 0, 0, 64));
}

#[test]
fn should_read_tag_values() {
    let values = [
        "#f00",
        "#F008",
        "#ff8000",
        "#ff800080",
        "red",
        "lightblue",
        "Red",
        "#ff80",
        "#xyz",
        "teal",
    ];

    assert_eq!(
        values.map(Color::from_tag_value),
        [
            Some(Color::new(255, 0, 0, 255)),
            Some(Color::new(255, 0, 0, 0x88)),
            Some(Color::new(255, 128, 0, 255)),
            Some(Color::new(255, 128, 0, 128)),
            Some(Color::new(255, 0, 0, 255)),
            Some(Color::new(173, 216, 230, 255)),
            None,
            Some(Color::new(255, 255, 136, 0)),
            None,
            None,
        ]
    );
}
