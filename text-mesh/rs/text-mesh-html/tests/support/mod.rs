use std::borrow::Cow;

use text_mesh_core::{Color, TextAssetTable, TextComponent};
use text_mesh_html::HtmlRenderer;

pub static ASSETS: TextAssetTable = TextAssetTable {
    fonts: Cow::Borrowed(&[Cow::Borrowed("Liberation's Sans")]),
    sprites: Cow::Borrowed(&[Cow::Borrowed("caret"), Cow::Borrowed("caret")]),
    ..TextAssetTable::EMPTY
};

pub fn component() -> TextComponent<'static> {
    TextComponent::new(Color::WHITE, 20.0, &ASSETS)
}

pub fn renderer() -> HtmlRenderer<'static> {
    HtmlRenderer::new(component())
}

pub fn render(text: &str) -> String {
    renderer().render(&component().parse(text))
}
