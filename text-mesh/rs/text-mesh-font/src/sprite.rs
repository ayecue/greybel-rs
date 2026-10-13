use std::borrow::Cow;

use crate::atlas::GlyphRect;
use crate::face::FaceInfo;
use crate::glyph::GlyphMetrics;

/// Sprite glyph's metrics, its sheet rectangle (y from the bottom, as Unity's `glyphRect`) and its scale.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpriteGlyph {
    pub metrics: GlyphMetrics,
    pub rect: GlyphRect,
    pub scale: f32,
}

/// Size of a sprite asset's sheet in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpriteTexture {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpriteCharacter {
    pub name: Cow<'static, str>,
    pub unicode: u32,
    pub glyph: usize,
    pub scale: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpriteAsset {
    pub name: Cow<'static, str>,
    pub face: FaceInfo,
    pub texture: Option<SpriteTexture>,
    pub glyphs: Cow<'static, [SpriteGlyph]>,
    pub characters: Cow<'static, [SpriteCharacter]>,
}

impl SpriteAsset {
    #[must_use]
    pub fn sprite(&self, index: usize) -> Option<(&SpriteCharacter, &SpriteGlyph)> {
        let character = self.characters.get(index)?;

        Some((character, self.glyphs.get(character.glyph)?))
    }
}
