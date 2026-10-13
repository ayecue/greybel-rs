#![allow(dead_code)]

pub mod sprites;

use std::borrow::Cow;

pub use text_mesh_core::Color;
use text_mesh_core::{
    LayoutChange, Length, Run, Style, Text, TextAssetTable, TextComponent, TextStyle,
};

pub const BASE_SIZE: f32 = 20.0;

pub static ASSETS: TextAssetTable = TextAssetTable {
    fonts: Cow::Borrowed(&[
        Cow::Borrowed("LiberationSans SDF"),
        Cow::Borrowed("SourceCodePro-Regular SDF"),
    ]),
    materials: Cow::Borrowed(&[
        Cow::Borrowed("LiberationSans SDF - Outline"),
        Cow::Borrowed("SourceCodePro-Regular SDF"),
    ]),
    sprites: Cow::Borrowed(&[
        Cow::Borrowed("caret"),
        Cow::Borrowed("caret"),
        Cow::Borrowed("caret"),
    ]),
    styles: Cow::Borrowed(&[
        TextStyle {
            name: Cow::Borrowed("H1"),
            open_markup: Cow::Borrowed("<size=2em><b><#40ff80>*"),
            close_markup: Cow::Borrowed("*</size></b></color>"),
        },
        TextStyle {
            name: Cow::Borrowed("C1"),
            open_markup: Cow::Borrowed("<color=#ffff40>"),
            close_markup: Cow::Borrowed("</color>"),
        },
        TextStyle {
            name: Cow::Borrowed("C3"),
            open_markup: Cow::Borrowed("<color=#80A0FF><b>"),
            close_markup: Cow::Borrowed("</color></b>"),
        },
        TextStyle {
            name: Cow::Borrowed("Escaped"),
            open_markup: Cow::Borrowed("\\u0041\\n"),
            close_markup: Cow::Borrowed(""),
        },
    ]),
};

pub fn component() -> TextComponent<'static> {
    component_in(Color::WHITE)
}

pub fn component_in(color: Color) -> TextComponent<'static> {
    TextComponent::new(color, BASE_SIZE, &ASSETS)
}

pub fn parse(text: &str) -> Text {
    component().parse(text)
}

pub fn plain(text: &str) -> String {
    parse(text).plain()
}

pub fn styles(text: &str) -> Vec<(String, Style)> {
    styles_in(&component(), text)
}

pub fn styles_in(component: &TextComponent, text: &str) -> Vec<(String, Style)> {
    component
        .parse(text)
        .runs
        .into_iter()
        .filter_map(|run| match run {
            Run::Text { text, style } => Some((text, style)),
            _ => None,
        })
        .collect()
}

pub fn style_of(text: &str, part: &str) -> Style {
    style_in(&component(), text, part)
}

pub fn style_in(component: &TextComponent, text: &str, part: &str) -> Style {
    styles_in(component, text)
        .into_iter()
        .find(|(run, _)| run.contains(part))
        .map(|(_, style)| style)
        .unwrap_or_else(|| panic!("no run contains {part:?} in {text:?}"))
}

pub fn pen_moves(text: &str) -> Vec<String> {
    parse(text)
        .runs
        .into_iter()
        .filter_map(|run| match run {
            Run::Position {
                x: Length::Pixels(x),
            } => Some(format!("pos {x:.2}px")),
            Run::Position {
                x: Length::Percentage(x),
            } => Some(format!("pos {x:.2}%")),
            Run::Space { width } => Some(format!("space {width:.2}px")),
            _ => None,
        })
        .collect()
}

pub fn layout_changes(text: &str) -> Vec<LayoutChange> {
    parse(text)
        .runs
        .into_iter()
        .filter_map(|run| match run {
            Run::Layout(change) => Some(change),
            _ => None,
        })
        .collect()
}
