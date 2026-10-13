use std::sync::LazyLock;

use text_mesh_font::FontSet;
use text_mesh_layout::{TerminalFrame, TextSettings};

/// Image with four RGBA bytes per pixel, ordered by row from top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u16,
    pub height: u16,
    pub rgba: &'static [u8],
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalText {
    pub font_set: FontSet,
    pub settings: TextSettings,
    pub frame: TerminalFrame,
    /// RGBA sheet for the terminal `caret` sprite.
    pub sprite_sheet: RgbaImage,
}

pub static TERMINAL_TEXT: LazyLock<TerminalText> = LazyLock::new(|| {
    let setup = &*crate::bundle::SETUP;
    let fonts = setup.font_set().expect("the game's setup gives every font");
    let texture = fonts
        .sprites
        .texture
        .expect("the terminal's sprite asset has a sheet");

    TerminalText {
        sprite_sheet: RgbaImage {
            width: texture.width,
            height: texture.height,
            rgba: crate::bundle::pixels("sprites", setup.sprite_sheet(), "rgba"),
        },
        font_set: fonts,
        settings: setup.text_settings(),
        frame: setup.frame().expect("the game's setup has a frame"),
    }
});

impl TerminalText {
    /// Returns TextMesh Pro setup JSON in `text-mesh-setup` format, including font generation, fallbacks, sprites,
    /// text settings, frame, and colors.
    #[must_use]
    pub fn game_setup() -> &'static str {
        crate::bundle::text("text-mesh.json")
    }
}
