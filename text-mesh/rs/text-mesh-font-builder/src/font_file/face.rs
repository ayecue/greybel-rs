use text_mesh_font::FaceInfo;

use super::{FontFile, HintedFont};
use crate::fixed_point::FixedPoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VerticalMetricTable {
    ascender: i64,
    descender: i64,
    line_gap: i64,
}

impl FontFile<'_> {
    const STRIKETHROUGH_TO_MEAN_LINE: f32 = 2.5;
    const SUB_AND_SUPERSCRIPT_SIZE: f32 = 0.5;

    /// Computes Unity-compatible face metrics with hinted cap and mean lines.
    #[must_use]
    pub fn face_info(&self, point_size: u16, hinted_font: Option<&HintedFont<'_>>) -> FaceInfo {
        let metrics = self.vertical_metrics();
        let units_per_pixel = self.units_per_em() as f32 / f32::from(point_size);
        let pixels = |units: i64| units as f32 / units_per_pixel;
        let (underline_position, underline_thickness) = self
            .face
            .tables()
            .post
            .map(|post| post.underline_metrics)
            .map_or((0, 0), |line| {
                (
                    i64::from(line.position) - i64::from(line.thickness) / 2,
                    i64::from(line.thickness),
                )
            });
        let line_top = |character: char| {
            hinted_font
                .zip(self.glyph_index(character))
                .and_then(|(hinted_font, glyph)| hinted_font.top(glyph))
                .map_or(0.0, |top| ((top + 63) >> 6) as f32)
        };
        let ascent_line = pixels(metrics.ascender);
        let descent_line = pixels(metrics.descender);
        let mean_line = line_top('x');

        FaceInfo {
            point_size: f32::from(point_size),
            scale: 1.0,
            line_height: pixels(metrics.ascender - metrics.descender + metrics.line_gap),
            ascent_line,
            cap_line: line_top('H'),
            mean_line,
            baseline: 0.0,
            descent_line,
            superscript_offset: ascent_line,
            superscript_size: Self::SUB_AND_SUPERSCRIPT_SIZE,
            subscript_offset: descent_line,
            subscript_size: Self::SUB_AND_SUPERSCRIPT_SIZE,
            underline_offset: pixels(underline_position),
            underline_thickness: pixels(underline_thickness),
            strikethrough_offset: mean_line / Self::STRIKETHROUGH_TO_MEAN_LINE,
            strikethrough_thickness: pixels(underline_thickness),
            tab_width: self.tab_width(point_size),
        }
    }

    fn tab_width(&self, point_size: u16) -> f32 {
        let Some(space_glyph) = self.glyph_index(' ') else {
            return 0.0;
        };
        let scale = FixedPoint::ratio(i64::from(point_size) * 64, self.units_per_em());
        let advance = scale.scale(self.advance(space_glyph));

        ((advance + 32) >> 6) as f32
    }

    /// Reads FreeType ascender, descender, and line gap from `hhea`, falling back to `OS/2` typographic values when unset.
    fn vertical_metrics(&self) -> VerticalMetricTable {
        let tables = self.face.tables();
        let hhea = VerticalMetricTable {
            ascender: i64::from(tables.hhea.ascender),
            descender: i64::from(tables.hhea.descender),
            line_gap: i64::from(tables.hhea.line_gap),
        };

        if hhea.ascender != 0 || hhea.descender != 0 {
            return hhea;
        }

        let Some(os2) = tables.os2 else {
            return hhea;
        };
        let typographic = VerticalMetricTable {
            ascender: i64::from(os2.typographic_ascender()),
            descender: i64::from(os2.typographic_descender()),
            line_gap: i64::from(os2.typographic_line_gap()),
        };

        if typographic.ascender != 0 || typographic.descender != 0 {
            return typographic;
        }

        VerticalMetricTable {
            ascender: i64::from(os2.windows_ascender()),
            descender: i64::from(os2.windows_descender()),
            line_gap: 0,
        }
    }
}
