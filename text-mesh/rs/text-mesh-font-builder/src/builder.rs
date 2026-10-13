use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use text_mesh_font::{
    FontAsset, FontAtlas, Glyph, GlyphAdjustment, GlyphRect, Kerning, KerningPair,
};

use crate::atlas::AtlasImage;
use crate::error::FontBuildError;
use crate::font_file::{FontFile, HintedFont};
use crate::packer::AtlasPacker;
use crate::render::{GlyphRenderer, RenderedGlyph};
use crate::render_plan::RenderPlan;
use crate::settings::FontBuildSettings;

/// Font asset (glyph table, kerning, material and atlas description) and its atlas images.
#[derive(Debug, Clone, PartialEq)]
pub struct BuiltFont {
    pub asset: FontAsset,
    pub atlases: Vec<AtlasImage>,
    /// Requested characters the font has no glyph for.
    pub unmapped_characters: Vec<char>,
}

/// Reproduces Unity Font Asset Creator font-file builds as a TMP font asset.
pub struct FontBuilder<'a> {
    font: FontFile<'a>,
    settings: FontBuildSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Placement {
    rect: GlyphRect,
    atlas: usize,
}

impl<'a> FontBuilder<'a> {
    /// # Errors
    /// Returns an error if the font data cannot be decoded.
    pub fn new(data: &'a [u8], settings: FontBuildSettings) -> Result<Self, FontBuildError> {
        Ok(Self {
            font: FontFile::parse(data)?,
            settings,
        })
    }

    #[must_use]
    pub const fn settings(&self) -> &FontBuildSettings {
        &self.settings
    }

    /// # Errors
    /// Returns an error if the atlas cannot fit the margin or a glyph.
    pub fn build(&self) -> Result<BuiltFont, FontBuildError> {
        let settings = &self.settings;
        let plan = RenderPlan::of(settings.render_mode);
        let margin = plan.packing_margin();

        if settings.atlas_width <= margin || settings.atlas_height <= margin {
            return Err(FontBuildError::AtlasSize {
                width: settings.atlas_width,
                height: settings.atlas_height,
            });
        }

        let hinted_font = HintedFont::new(self.font.data(), settings.point_size);
        let (characters, unmapped_characters) = self.characters();
        let mut glyph_order: Vec<u16> = Vec::new();

        for &(_, glyph) in &characters {
            if !glyph_order.contains(&glyph) {
                glyph_order.push(glyph);
            }
        }

        let renderer = GlyphRenderer::new(
            &self.font,
            hinted_font.as_ref(),
            plan,
            settings.point_size,
            settings.padding,
        );
        let rendered_glyphs: Vec<RenderedGlyph> = glyph_order
            .iter()
            .map(|&glyph| renderer.render(glyph))
            .collect();
        let placements = self.pack(&rendered_glyphs, margin)?;
        let atlases = self.draw(&rendered_glyphs, &placements);
        let by_glyph_id: BTreeMap<u16, (&RenderedGlyph, Placement)> = rendered_glyphs
            .iter()
            .zip(&placements)
            .map(|(glyph, placement)| (glyph.glyph_id, (glyph, *placement)))
            .collect();
        let glyphs = characters.iter().map(|&(character, glyph)| {
            let (rendered_glyph, placement) = by_glyph_id[&glyph];

            Glyph {
                character,
                metrics: rendered_glyph.metrics,
                rect: placement.rect,
            }
        });
        let kerning = if settings.kerning {
            self.kerning(&characters)
        } else {
            Vec::new()
        };
        let asset = FontAsset {
            name: Cow::Owned(settings.name.clone()),
            face: self
                .font
                .face_info(settings.point_size, hinted_font.as_ref()),
            style: settings.style,
            glyphs: Cow::Borrowed(&[]),
            kerning: Cow::Borrowed(&[]),
            fallbacks: Cow::Borrowed(&[]),
            material: settings.material(),
            atlas: Some(FontAtlas {
                width: settings.atlas_width,
                height: settings.atlas_height,
                padding: settings.padding,
                render_mode: settings.render_mode,
                count: u8::try_from(atlases.len()).unwrap_or(u8::MAX),
            }),
        }
        .with_glyphs(glyphs)
        .with_kerning(kerning);

        Ok(BuiltFont {
            asset,
            atlases,
            unmapped_characters,
        })
    }

    /// Requested characters with their glyphs, first occurrences only,
    /// and the ones the font lacks.
    fn characters(&self) -> (Vec<(char, u16)>, Vec<char>) {
        let mut character_set = BTreeSet::new();
        let mut characters = Vec::new();
        let mut unmapped_characters = Vec::new();

        for &character in &self.settings.characters {
            if !character_set.insert(character) {
                continue;
            }

            match self.font.glyph_index(character) {
                Some(glyph) => characters.push((character, glyph)),
                None => unmapped_characters.push(character),
            }
        }

        (characters, unmapped_characters)
    }

    /// Packs glyphs in `glyph_order` and opens another atlas when full; rectangle size includes twice the padding plus margin.
    fn pack(
        &self,
        rendered_glyphs: &[RenderedGlyph],
        margin: u16,
    ) -> Result<Vec<Placement>, FontBuildError> {
        let settings = &self.settings;
        let (width, height) = (
            u32::from(settings.atlas_width - margin),
            u32::from(settings.atlas_height - margin),
        );
        let grow = 2 * u32::from(settings.padding) + u32::from(margin);
        let offset = u32::from(settings.padding) + u32::from(margin);
        let mut placements = vec![
            Placement {
                rect: GlyphRect::default(),
                atlas: 0,
            };
            rendered_glyphs.len()
        ];
        let mut unplaced_indices: Vec<usize> = (0..rendered_glyphs.len())
            .filter(|&index| !rendered_glyphs[index].is_empty())
            .collect();
        let mut atlas = 0;

        while !unplaced_indices.is_empty() {
            let mut packer = AtlasPacker::new(width, height);
            let mut glyphs_to_pack = Vec::new();

            for &index in &unplaced_indices {
                let glyph = &rendered_glyphs[index];
                let (glyph_width, glyph_height) = (
                    u32::from(glyph.width) + grow,
                    u32::from(glyph.height) + grow,
                );

                match packer.insert(glyph_width, glyph_height) {
                    Some(packed_rect) => {
                        placements[index] = Placement {
                            rect: GlyphRect {
                                x: u16::try_from(packed_rect.x + offset).unwrap_or(u16::MAX),
                                y: u16::try_from(packed_rect.y + offset).unwrap_or(u16::MAX),
                                width: glyph.width,
                                height: glyph.height,
                                atlas: u8::try_from(atlas).unwrap_or(u8::MAX),
                            },
                            atlas,
                        };
                    }
                    None if glyph_width > width || glyph_height > height => {
                        return Err(FontBuildError::GlyphTooLarge {
                            glyph: glyph.glyph_id,
                            width: glyph_width,
                            height: glyph_height,
                        });
                    }
                    None => glyphs_to_pack.push(index),
                }
            }

            unplaced_indices = glyphs_to_pack;
            atlas += 1;
        }

        Ok(placements)
    }

    fn draw(&self, rendered_glyphs: &[RenderedGlyph], placements: &[Placement]) -> Vec<AtlasImage> {
        let count = placements
            .iter()
            .zip(rendered_glyphs)
            .filter(|(_, glyph)| !glyph.is_empty())
            .map(|(placement, _)| placement.atlas + 1)
            .max()
            .unwrap_or(1);
        let mut atlases =
            vec![AtlasImage::new(self.settings.atlas_width, self.settings.atlas_height); count];
        let padding = usize::from(self.settings.padding);

        for (glyph, placement) in rendered_glyphs.iter().zip(placements) {
            if glyph.is_empty() {
                continue;
            }

            atlases[placement.atlas].draw(
                usize::from(placement.rect.x) - padding,
                usize::from(placement.rect.y) - padding,
                glyph.padded_width(),
                glyph.padded_height(),
                &glyph.pixels,
            );
        }

        atlases
    }

    /// Returns pixel kerning at the sampling size between character glyphs, stored as the first glyph's advance adjustment.
    fn kerning(&self, characters: &[(char, u16)]) -> Vec<Kerning> {
        let mut by_glyph_id: BTreeMap<u16, Vec<char>> = BTreeMap::new();

        for &(character, glyph) in characters {
            by_glyph_id.entry(glyph).or_default().push(character);
        }

        let glyphs: BTreeSet<u16> = by_glyph_id.keys().copied().collect();
        let pairs = self.font.kerning(&glyphs);
        let units_per_pixel = self.font.units_per_em() as f32 / f32::from(self.settings.point_size);
        let mut kerning = Vec::new();

        for (&(first_glyph, second_glyph), &units) in &pairs {
            for &first in &by_glyph_id[&first_glyph] {
                for &second in &by_glyph_id[&second_glyph] {
                    kerning.push(Kerning {
                        first,
                        second,
                        pair: KerningPair {
                            first: GlyphAdjustment {
                                x_advance: units as f32 / units_per_pixel,
                                ..GlyphAdjustment::default()
                            },
                            second: GlyphAdjustment::default(),
                        },
                    });
                }
            }
        }

        kerning
    }
}
