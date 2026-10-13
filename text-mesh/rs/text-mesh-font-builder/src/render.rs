use text_mesh_font::GlyphMetrics;

use crate::distance::{AntiAliasedDistance, SampledDistance};
use crate::fixed_point::FixedPoint;
use crate::font_file::{FontFile, HintedFont};
use crate::outline::Outline;
use crate::raster::{CoverageRaster, FlatPath, MonochromeRaster, PixelBox, Subdivision};
use crate::render_plan::{DistanceField, Rasterization, RenderPlan};

/// Glyph_id as the font engine renders it into the atlas: its metrics, the size of its rectangle, and the padded pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedGlyph {
    pub glyph_id: u16,
    pub metrics: GlyphMetrics,
    pub width: u16,
    pub height: u16,
    pub padding: u16,
    pub pixels: Vec<u8>,
}

impl RenderedGlyph {
    #[must_use]
    pub fn padded_width(&self) -> usize {
        usize::from(self.width) + 2 * usize::from(self.padding)
    }

    #[must_use]
    pub fn padded_height(&self) -> usize {
        usize::from(self.height) + 2 * usize::from(self.padding)
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

pub struct GlyphRenderer<'f, 'a> {
    font: &'f FontFile<'a>,
    hinted_font: Option<&'f HintedFont<'a>>,
    plan: RenderPlan,
    padding: u16,
    scale: FixedPoint,
}

impl<'f, 'a> GlyphRenderer<'f, 'a> {
    #[must_use]
    pub fn new(
        font: &'f FontFile<'a>,
        hinted_font: Option<&'f HintedFont<'a>>,
        plan: RenderPlan,
        point_size: u16,
        padding: u16,
    ) -> Self {
        let pixels = i64::from(point_size) * i64::from(plan.upsampling) * 64;

        Self {
            font,
            hinted_font,
            plan,
            padding,
            scale: FixedPoint::ratio(pixels, font.units_per_em()),
        }
    }

    #[must_use]
    pub fn render(&self, glyph_id: u16) -> RenderedGlyph {
        let (outline, metrics) = match self.hinted_font.filter(|_| self.plan.hinted) {
            Some(hinted_font) => self.hinted_outline(hinted_font, glyph_id),
            None => self.outline(glyph_id),
        };
        let (width, height, pixels) = match self.plan.rasterization {
            Rasterization::Monochrome => self.monochrome(&outline),
            Rasterization::Coverage => self.coverage(&outline),
        };

        RenderedGlyph {
            glyph_id,
            metrics,
            width,
            height,
            padding: self.padding,
            pixels,
        }
    }

    fn upsampling(&self) -> i64 {
        if self.plan.hinted {
            1
        } else {
            i64::from(self.plan.upsampling)
        }
    }

    /// Returns the unhinted outline scaled by sampling size and upsampling, with FreeType pixel metrics.
    fn outline(&self, glyph_id: u16) -> (Outline, GlyphMetrics) {
        let outline = self.font.outline(glyph_id, self.scale);
        let unit = 64.0 * self.upsampling() as f64;
        let pixels = |value: i64| (value as f64 / unit) as f32;
        let advance = pixels(self.scale.scale(self.font.advance(glyph_id)));
        let metrics = outline.control_box().map_or(
            GlyphMetrics {
                advance,
                ..GlyphMetrics::default()
            },
            |bounds| GlyphMetrics {
                width: pixels(bounds.x_max - bounds.x_min),
                height: pixels(bounds.y_max - bounds.y_min),
                bearing_x: pixels(bounds.x_min),
                bearing_y: pixels(bounds.y_max),
                advance,
            },
        );

        (outline, metrics)
    }

    /// Returns the hinted outline with FreeType grid-fitted metrics rounded to whole pixels.
    fn hinted_outline(
        &self,
        hinted_font: &HintedFont<'_>,
        glyph_id: u16,
    ) -> (Outline, GlyphMetrics) {
        let Some(hinted_glyph) = hinted_font.glyph(glyph_id) else {
            return self.outline(glyph_id);
        };
        let linear = self.scale.scale(self.font.advance(glyph_id));
        let advance = ((hinted_glyph.advance.unwrap_or(linear) + 32) >> 6) as f32;
        let metrics = hinted_glyph.outline.control_box().map_or(
            GlyphMetrics {
                advance,
                ..GlyphMetrics::default()
            },
            |bounds| {
                let left = bounds.x_min.div_euclid(64);
                let right = (bounds.x_max + 63).div_euclid(64);
                let bottom = bounds.y_min.div_euclid(64);
                let top = (bounds.y_max + 63).div_euclid(64);

                GlyphMetrics {
                    width: (right - left) as f32,
                    height: (top - bottom) as f32,
                    bearing_x: left as f32,
                    bearing_y: top as f32,
                    advance,
                }
            },
        );

        (hinted_glyph.outline, metrics)
    }

    fn monochrome(&self, outline: &Outline) -> (u16, u16, Vec<u8>) {
        let upsampling = self.upsampling();
        let padding = i64::from(self.padding);
        let bounds = PixelBox::monochrome(outline.control_box());
        let width = (bounds.width + upsampling - 1) / upsampling;
        let height = (bounds.height + upsampling - 1) / upsampling;
        let (padded_width, padded_height) = (width + 2 * padding, height + 2 * padding);
        let path = FlatPath::new(outline, Subdivision::Precise);
        let raster = MonochromeRaster::new(
            &path,
            (bounds.left - padding * upsampling) as f64,
            (bounds.top + padding * upsampling) as f64,
            (padded_width * upsampling) as usize,
            (padded_height * upsampling) as usize,
        );
        let pixels = match self.plan.distance {
            DistanceField::Bitmap | DistanceField::Coverage => {
                SampledDistance::new(&raster, upsampling as u16, self.padding + 1)
                    .alpha(padded_width as usize, padded_height as usize)
            }
            DistanceField::None => (0..padded_height as usize)
                .flat_map(|row| (0..padded_width as usize).map(move |column| (column, row)))
                .map(|(column, row)| if raster.is_set(column, row) { 255 } else { 0 })
                .collect(),
        };

        (Self::to_u16(width), Self::to_u16(height), pixels)
    }

    fn coverage(&self, outline: &Outline) -> (u16, u16, Vec<u8>) {
        let padding = i64::from(self.padding);
        let bounds = PixelBox::smooth(outline.control_box());
        let (padded_width, padded_height) = (
            (bounds.width + 2 * padding) as usize,
            (bounds.height + 2 * padding) as usize,
        );
        let path = FlatPath::new(outline, Subdivision::FreeTypeGray);
        let raster = CoverageRaster::new(
            &path,
            (bounds.left - padding) as f64,
            (bounds.top + padding) as f64,
            padded_width,
            padded_height,
        );
        let pixels = match self.plan.distance {
            DistanceField::Coverage | DistanceField::Bitmap => {
                AntiAliasedDistance::new(padded_width, padded_height, self.padding + 1)
                    .alpha(raster.alpha())
            }
            DistanceField::None => raster.alpha().to_vec(),
        };

        (
            Self::to_u16(bounds.width),
            Self::to_u16(bounds.height),
            pixels,
        )
    }

    fn to_u16(pixels: i64) -> u16 {
        u16::try_from(pixels.max(0)).unwrap_or(u16::MAX)
    }
}
