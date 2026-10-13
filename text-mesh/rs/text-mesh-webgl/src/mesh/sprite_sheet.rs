use std::borrow::Cow;

use text_mesh_font::{GlyphRect, SpriteAsset};

/// Sprite sheet's size and the rect of every sprite glyph, y from the bottom as Unity's `glyphRect`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpriteSheet {
    pub width: u16,
    pub height: u16,
    pub rects: Cow<'static, [GlyphRect]>,
}

impl SpriteSheet {
    /// Sheet of `sprites`, when its texture size is known.
    #[must_use]
    pub fn from_asset(sprites: &SpriteAsset) -> Option<Self> {
        let texture = sprites.texture?;

        Some(Self {
            width: texture.width,
            height: texture.height,
            rects: sprites.glyphs.iter().map(|glyph| glyph.rect).collect(),
        })
    }

    /// Returns `TMP_Text.SaveSpriteVertexInfo` UVs in bottom-left, top-left, top-right, bottom-right order.
    #[must_use]
    pub fn uvs(&self, glyph: usize) -> Option<[[f32; 2]; 4]> {
        let rect = self.rects.get(glyph)?;
        let width = f32::from(self.width);
        let height = f32::from(self.height);
        let left = f32::from(rect.x) / width;
        let bottom = f32::from(rect.y) / height;
        let top = f32::from(rect.y + rect.height) / height;
        let right = f32::from(rect.x + rect.width) / width;

        Some([[left, bottom], [left, top], [right, top], [right, bottom]])
    }
}
