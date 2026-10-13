use text_mesh_font::{FontSet, SdfMaterial};

use crate::layout::ElementKind;
use crate::text_layout::source::SourceElement;

const SMALL_CAPS_SCALE: f32 = 0.8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ElementMetrics {
    pub character: u32,
    pub font: usize,
    pub material: Option<SdfMaterial>,
    pub scale: f32,
    pub small_caps: f32,
    pub ascent_line: f32,
    pub descent_line: f32,
    pub element_baseline: f32,
    pub font_scale_multiplier: f32,
    pub baseline_offset: f32,
    pub padding: f32,
}

impl ElementMetrics {
    pub(crate) fn new(fonts: &FontSet, element: &SourceElement, current_font: usize) -> Self {
        let style = &element.style;
        let size = style.size;
        let base = element.style_font.unwrap_or(fonts.primary);
        let (font_scale_multiplier, baseline_offset) = Self::script_offsets(fonts, element, base);

        match element.kind {
            ElementKind::Sprite { .. } => {
                let font = fonts.font(current_font);
                let face = &font.face;
                let sprites = &fonts.sprites.face;
                let font_scale = size / face.point_size * face.scale;
                let (scale, ascent_line, element_baseline, descent_line) =
                    if sprites.point_size > 0.0 {
                        let sprite_scale = size / sprites.point_size * sprites.scale;

                        (
                            element.element_scale * element.glyph_scale * sprite_scale,
                            sprites.ascent_line,
                            sprites.baseline * font_scale * font_scale_multiplier * sprites.scale,
                            sprites.descent_line,
                        )
                    } else {
                        let sprite_scale = size / face.point_size * face.scale;
                        let scale = face.ascent_line / element.glyph.height
                            * element.element_scale
                            * element.glyph_scale
                            * sprite_scale;
                        let delta = sprite_scale / scale;

                        (
                            scale,
                            face.ascent_line * delta,
                            face.baseline * font_scale * font_scale_multiplier * face.scale,
                            face.descent_line * delta,
                        )
                    };

                Self {
                    character: element.unicode(),
                    font: current_font,
                    material: font.material,
                    scale,
                    small_caps: 1.0,
                    ascent_line,
                    descent_line,
                    element_baseline,
                    font_scale_multiplier,
                    baseline_offset,
                    padding: 0.0,
                }
            }
            ElementKind::Glyph { font, .. } => {
                let asset = fonts.font(font);
                let face = &asset.face;
                let small_caps = if style.small_caps {
                    SMALL_CAPS_SCALE
                } else {
                    1.0
                };
                let size_scale = size * small_caps / face.point_size * face.scale;
                let material = Self::material(fonts, base, font);

                Self {
                    character: element.unicode(),
                    font,
                    material,
                    scale: size_scale
                        * font_scale_multiplier
                        * element.element_scale
                        * element.glyph_scale,
                    small_caps,
                    ascent_line: face.ascent_line,
                    descent_line: face.descent_line,
                    element_baseline: face.baseline
                        * size_scale
                        * font_scale_multiplier
                        * face.scale,
                    font_scale_multiplier,
                    baseline_offset,
                    padding: material.map_or(1.0, |material| material.padding()),
                }
            }
        }
    }

    fn material(fonts: &FontSet, base: usize, font: usize) -> Option<SdfMaterial> {
        let target = fonts.font(font).material;

        if font == fonts.primary || font == base {
            return target;
        }

        match (fonts.font(base).material, target) {
            (Some(source), Some(target)) => Some(source.with_atlas_of(&target)),
            (_, target) => target,
        }
    }

    fn script_offsets(fonts: &FontSet, element: &SourceElement, base: usize) -> (f32, f32) {
        let style = &element.style;
        let face = &fonts.font(base).face;
        let font_scale = style.size / face.point_size * face.scale;
        let mut multiplier = 1.0;
        let mut offset = style.vertical_offset;

        if style.subscript {
            multiplier *= if face.subscript_size > 0.0 {
                face.subscript_size
            } else {
                1.0
            };
            offset += face.subscript_offset * font_scale * multiplier;
        }

        if style.superscript {
            multiplier *= if face.superscript_size > 0.0 {
                face.superscript_size
            } else {
                1.0
            };
            offset += face.superscript_offset * font_scale * multiplier;
        }

        (multiplier, offset)
    }

    pub(crate) fn is_white_space(&self) -> bool {
        self.character <= 0xFFFF && char::from_u32(self.character).is_some_and(char::is_whitespace)
    }
}
