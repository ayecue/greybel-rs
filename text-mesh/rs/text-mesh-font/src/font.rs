use std::borrow::Cow;

use crate::atlas::{FontAtlas, GlyphRect};
use crate::face::{FaceInfo, FontStyleSettings};
use crate::glyph::{Glyph, GlyphMetrics, Kerning, KerningPair};
use crate::material::SdfMaterial;

/// Character-sorted `glyphs` and pair-sorted `kerning` support binary-search lookups without hash tables.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FontAsset {
    pub name: Cow<'static, str>,
    pub face: FaceInfo,
    pub style: FontStyleSettings,
    pub glyphs: Cow<'static, [Glyph]>,
    pub kerning: Cow<'static, [Kerning]>,
    pub fallbacks: Cow<'static, [usize]>,
    pub material: Option<SdfMaterial>,
    pub atlas: Option<FontAtlas>,
}

impl FontAsset {
    const CHARACTERS_WITH_SYNTHETIC_METRICS: [char; 12] = [
        '\u{3}', '\t', '\n', '\u{b}', '\r', '\u{61c}', '\u{200b}', '\u{200e}', '\u{200f}',
        '\u{2028}', '\u{2029}', '\u{2060}',
    ];

    #[must_use]
    pub fn with_glyphs(self, glyphs: impl IntoIterator<Item = Glyph>) -> Self {
        let mut glyphs: Vec<Glyph> = glyphs.into_iter().collect();

        glyphs.sort_by_key(|glyph| glyph.character);
        glyphs.dedup_by_key(|glyph| glyph.character);

        Self {
            glyphs: Cow::Owned(glyphs),
            ..self
        }
    }

    #[must_use]
    pub fn with_kerning(self, kerning: impl IntoIterator<Item = Kerning>) -> Self {
        let mut kerning: Vec<Kerning> = kerning.into_iter().collect();

        kerning.sort_by_key(|pair| (pair.first, pair.second));
        kerning.dedup_by_key(|pair| (pair.first, pair.second));

        Self {
            kerning: Cow::Owned(kerning),
            ..self
        }
    }

    #[must_use]
    pub fn is_sorted(&self) -> bool {
        self.glyphs
            .windows(2)
            .all(|pair| pair[0].character < pair[1].character)
            && self
                .kerning
                .windows(2)
                .all(|pair| (pair[0].first, pair[0].second) < (pair[1].first, pair[1].second))
    }

    #[must_use]
    pub fn glyph(&self, character: char) -> Option<GlyphMetrics> {
        self.own_glyph(character).or_else(|| {
            Self::CHARACTERS_WITH_SYNTHETIC_METRICS
                .contains(&character)
                .then(GlyphMetrics::default)
        })
    }

    #[must_use]
    pub fn kerning(&self, first: char, second: char) -> Option<&KerningPair> {
        self.kerning
            .binary_search_by_key(&(first, second), |pair| (pair.first, pair.second))
            .ok()
            .map(|index| &self.kerning[index].pair)
    }

    #[must_use]
    pub fn cap_line(&self) -> f32 {
        if self.face.cap_line == 0.0
            && let Some(glyph) = self.own_glyph('X')
        {
            return glyph.bearing_y;
        }

        self.face.cap_line
    }

    #[must_use]
    pub fn padding(&self) -> f32 {
        self.material.map_or(1.0, |material| material.padding())
    }

    #[must_use]
    pub fn rect(&self, character: char) -> Option<GlyphRect> {
        self.find(character).map(|glyph| glyph.rect)
    }

    fn own_glyph(&self, character: char) -> Option<GlyphMetrics> {
        self.find(character).map(|glyph| glyph.metrics)
    }

    fn find(&self, character: char) -> Option<&Glyph> {
        self.glyphs
            .binary_search_by_key(&character, |glyph| glyph.character)
            .ok()
            .map(|index| &self.glyphs[index])
    }
}
