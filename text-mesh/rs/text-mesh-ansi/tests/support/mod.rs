#![allow(dead_code)]

pub mod layout;

use std::borrow::Cow;

use text_mesh_ansi::AnsiRenderer;
use text_mesh_core::{Color, TextAssetTable, TextComponent};

pub static ASSETS: TextAssetTable = TextAssetTable {
    sprites: Cow::Borrowed(&[Cow::Borrowed("caret")]),
    ..TextAssetTable::EMPTY
};

pub fn component() -> TextComponent<'static> {
    TextComponent::new(Color::WHITE, 20.0, &ASSETS)
}

pub fn renderer() -> AnsiRenderer<'static> {
    AnsiRenderer::new(component(), Color::BLACK)
}

pub fn render(text: &str) -> String {
    renderer().render(&component().parse(text))
}

pub fn render_painted(text: &str) -> String {
    renderer()
        .with_painted_background()
        .render(&component().parse(text))
}

pub fn render_laid_out(text: &str, columns: usize) -> String {
    let setup = text_mesh_test_support::setup();
    let component = TextComponent::new(Color::WHITE, setup.settings.font_size, &setup.assets);

    AnsiRenderer::new(component, Color::BLACK)
        .with_cells(setup.cells())
        .laid_out(setup.text_layout(), columns)
        .render(&component.parse(text))
}
