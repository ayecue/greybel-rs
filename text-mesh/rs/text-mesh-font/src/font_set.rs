use std::borrow::Cow;

use crate::font::FontAsset;
use crate::glyph::GlyphMetrics;
use crate::sprite::SpriteAsset;

#[derive(Debug, Clone, PartialEq)]
pub struct FontSet {
    pub fonts: Cow<'static, [FontAsset]>,
    pub primary: usize,
    pub global_fallbacks: Cow<'static, [usize]>,
    pub missing_glyph: char,
    pub sprites: SpriteAsset,
}

impl FontSet {
    #[must_use]
    pub fn font(&self, index: usize) -> &FontAsset {
        &self.fonts[index]
    }

    #[must_use]
    pub fn find(&self, name: &str) -> Option<usize> {
        self.fonts
            .iter()
            .position(|font| font.name.eq_ignore_ascii_case(name))
    }

    #[must_use]
    pub fn glyph(&self, font: usize, character: char) -> Option<(usize, GlyphMetrics)> {
        if let Some(glyph) = self.fonts[font].glyph(character) {
            return Some((font, glyph));
        }

        let fonts = &self.fonts[font].fallbacks;

        if let Some(font_glyph) = self.search_list(fonts, character) {
            return Some(font_glyph);
        }

        if font != self.primary {
            if let Some(glyph) = self.fonts[self.primary].glyph(character) {
                return Some((self.primary, glyph));
            }

            let fonts = &self.fonts[self.primary].fallbacks;

            if let Some(font_glyph) = self.search_list(fonts, character) {
                return Some(font_glyph);
            }
        }

        self.search_list(&self.global_fallbacks, character)
            .or_else(|| self.search(self.primary, character))
    }

    #[must_use]
    pub fn element(&self, font: usize, character: char) -> (char, usize, GlyphMetrics) {
        if let Some((font_index, glyph)) = self.glyph(font, character) {
            return (character, font_index, glyph);
        }

        let replacement = self.missing_glyph;
        let font_glyph = self
            .search(font, replacement)
            .or_else(|| self.search_list(&self.global_fallbacks, replacement))
            .or_else(|| self.search(self.primary, replacement));

        if let Some((font_glyph, glyph)) = font_glyph {
            return (replacement, font_glyph, glyph);
        }

        if let Some((font_glyph, glyph)) = self.search(font, ' ') {
            return (' ', font_glyph, glyph);
        }

        let end_of_text = '\u{3}';
        let (font_glyph, glyph) = self
            .search(font, end_of_text)
            .unwrap_or((font, GlyphMetrics::default()));

        (end_of_text, font_glyph, glyph)
    }

    fn search(&self, font: usize, character: char) -> Option<(usize, GlyphMetrics)> {
        self.search_from(font, character, &mut Vec::new())
    }

    fn search_list(&self, fonts: &[usize], character: char) -> Option<(usize, GlyphMetrics)> {
        let mut searched_fonts = Vec::new();

        fonts
            .iter()
            .find_map(|&font| self.search_from(font, character, &mut searched_fonts))
    }

    fn search_from(
        &self,
        font: usize,
        character: char,
        searched_fonts: &mut Vec<usize>,
    ) -> Option<(usize, GlyphMetrics)> {
        let asset = &self.fonts[font];

        if let Some(glyph) = asset.glyph(character) {
            return Some((font, glyph));
        }

        for &fallback in asset.fallbacks.iter() {
            if searched_fonts.contains(&fallback) {
                continue;
            }

            searched_fonts.push(fallback);
            if let Some(font_glyph) = self.search_from(fallback, character, searched_fonts) {
                return Some(font_glyph);
            }
        }

        None
    }
}
