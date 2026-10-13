const LINE_GAP_ADJUSTMENT: f32 = 0.045;

use std::sync::LazyLock;

use text_mesh_core::{CellMetricContainer, Color, TextAssetTable, TextComponent};

use crate::TERMINAL_TEXT;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalTheme {
    pub text: Color,
    pub background: Color,
    pub font_size: f32,
    pub font: &'static str,
    pub cells: CellMetricContainer,
}

/// Default terminal text and background colors, font family and size, and cell metrics.
pub static TERMINAL_THEME: LazyLock<TerminalTheme> = LazyLock::new(|| {
    let (text, background) = crate::bundle::SETUP
        .colors()
        .expect("the game's setup has colours");
    let fonts = &TERMINAL_TEXT.font_set;
    let font = &fonts.fonts[fonts.primary];
    let space_glyph = font
        .glyph(' ')
        .expect("the terminal font has a space_glyph");
    let point_size = font.face.point_size;

    TerminalTheme {
        text,
        background,
        font_size: TERMINAL_TEXT.settings.font_size,
        font: &font.name,
        cells: CellMetricContainer {
            advance: space_glyph.advance / point_size,
            line_height: font.face.line_height / point_size + LINE_GAP_ADJUSTMENT,
        },
    }
});

impl TerminalTheme {
    #[must_use]
    pub const fn component<'a>(&self, assets: &'a TextAssetTable) -> TextComponent<'a> {
        TextComponent::new(self.text, self.font_size, assets)
    }
}
