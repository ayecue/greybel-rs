//! What the game replaces before parsing tags.

use text_mesh_core::{Color, Run};

use crate::support::{parse, plain, style_of};

#[test]
fn should_break_the_line_at_br_keeping_the_style() {
    let text = "<b>a<br>b";
    let runs: Vec<String> = parse(text)
        .runs
        .into_iter()
        .map(|run| match run {
            Run::Text { text, style } => format!("{text}:{}", style.bold),
            Run::LineBreak => "|".to_owned(),
            _ => "other".to_owned(),
        })
        .collect();

    assert_eq!(runs, ["a:true", "|", "b:true"]);
}

#[test]
fn should_replace_br_in_any_case_and_skip_four_characters() {
    let text = "a<BR>b<br=x>c";
    let shown = plain(text);

    assert_eq!(shown, "a\nb\nx>c");
}

#[test]
fn should_keep_a_br_at_the_end_of_the_text() {
    let text = "a<br";
    let shown = plain(text);

    assert_eq!(shown, "a<br");
}

#[test]
fn should_replace_br_inside_noparse() {
    let text = "<noparse>a<br>b</noparse>";
    let shown = plain(text);

    assert_eq!(shown, "a\nb");
}

#[test]
fn should_replace_the_space_tags() {
    let text = "a<nbsp>b<zwsp>c";
    let shown = plain(text);

    assert_eq!(shown, "a\u{a0}b\u{200b}c");
}

#[test]
fn should_drop_an_unmatched_closing_style_and_show_an_unknown_style_as_text() {
    let text = "a</style>b<style=H4>c";
    let shown = plain(text);

    assert_eq!(shown, "ab<style=H4>c");
}

#[test]
fn should_insert_the_tags_of_a_style_from_the_games_style_sheet() {
    let text = "<style=h1>a</style>b";
    let shown = (plain(text), style_of(text, "a"), style_of(text, "b"));

    assert_eq!(
        (
            shown.0.as_str(),
            shown.1.bold,
            shown.1.size,
            shown.1.color,
            shown.2.bold,
            shown.2.size
        ),
        (
            "*a*b",
            true,
            40.0,
            Color::new(0x40, 0xff, 0x80, 255),
            false,
            20.0
        )
    );
}

#[test]
fn should_read_the_escapes_in_a_style() {
    let text = "<style=Escaped>b";
    let shown = plain(text);

    assert_eq!(shown, "A\nb");
}

#[test]
fn should_show_a_style_as_text_without_a_style_sheet() {
    let assets = text_mesh_core::TextAssetTable::EMPTY;
    let component = text_mesh_core::TextComponent::new(Color::WHITE, 20.0, &assets);
    let shown = component.parse("<style=H1>a</style>").plain();

    assert_eq!(shown, "<style=H1>a");
}

#[test]
fn should_close_nested_styles_in_reverse_order() {
    let text = "<style=\"C3\"><style=C1>a</style>b</style>c";
    let shown = ["a", "b", "c"].map(|part| style_of(text, part).color);

    assert_eq!(
        shown,
        [
            Color::new(0xff, 0xff, 0x40, 255),
            Color::new(0x80, 0xa0, 0xff, 255),
            Color::WHITE
        ]
    );
}

#[test]
fn should_end_the_text_at_a_null_character() {
    let text = "a\0b";
    let shown = plain(text);

    assert_eq!(shown, "a");
}
