mod face;
mod hinting;
mod kerning;
mod truetype;

pub use hinting::HintedFont;
use truetype::TrueTypeLoader;
use ttf_parser::{Face, GlyphId, OutlineBuilder, Tag};

use crate::error::FontBuildError;
use crate::fixed_point::FixedPoint;
use crate::outline::{Outline, PointKind};

/// Parsed TrueType or OpenType font, read as Unity's FontEngine (FreeType) reads it.
#[derive(Clone)]
pub struct FontFile<'a> {
    data: &'a [u8],
    face: Face<'a>,
    truetype: Option<TrueTypeLoader<'a>>,
}

struct ScaledOutline {
    outline: Outline,
    scale: f64,
}

impl ScaledOutline {
    fn point(&mut self, x: f32, y: f32, kind: PointKind) {
        let scale = |value: f32| (f64::from(value) * self.scale).round() as i64;
        let (x, y) = (scale(x), scale(y));

        self.outline.push_point(x, y, kind);
    }
}

impl OutlineBuilder for ScaledOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.outline.close_contour();
        self.point(x, y, PointKind::OnCurve);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.point(x, y, PointKind::OnCurve);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.point(x1, y1, PointKind::Quadratic);
        self.point(x, y, PointKind::OnCurve);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.point(x1, y1, PointKind::Cubic);
        self.point(x2, y2, PointKind::Cubic);
        self.point(x, y, PointKind::OnCurve);
    }

    fn close(&mut self) {
        self.outline.close_contour();
    }
}

impl<'a> FontFile<'a> {
    const NON_BREAKING_HYPHEN: char = '\u{2011}';
    const NO_BREAK_SPACE: char = '\u{a0}';
    const SOFT_HYPHEN: char = '\u{ad}';

    /// # Errors
    /// Returns an error if ttf-parser cannot decode the font data.
    pub fn parse(data: &'a [u8]) -> Result<Self, FontBuildError> {
        let face = Face::parse(data, 0).map_err(FontBuildError::Font)?;
        let raw = face.raw_face();
        let truetype = match (
            raw.table(Tag::from_bytes(b"loca")),
            raw.table(Tag::from_bytes(b"glyf")),
        ) {
            (Some(locations), Some(glyphs)) => Some(TrueTypeLoader::new(
                locations,
                glyphs,
                face.tables().head.index_to_location_format
                    == ttf_parser::head::IndexToLocationFormat::Long,
            )),
            _ => None,
        };

        Ok(Self {
            data,
            face,
            truetype,
        })
    }

    #[must_use]
    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    #[must_use]
    pub fn units_per_em(&self) -> i64 {
        i64::from(self.face.units_per_em())
    }

    /// Glyph TMP uses for a character; missing no-break spaces and hyphens fall back to the plain space and hyphen-minus.
    #[must_use]
    pub fn glyph_index(&self, character: char) -> Option<u16> {
        let own = self.face.glyph_index(character).map(|glyph| glyph.0);

        own.filter(|&glyph| glyph != 0).or_else(|| match character {
            Self::NO_BREAK_SPACE => self.glyph_index(' '),
            Self::SOFT_HYPHEN | Self::NON_BREAKING_HYPHEN => self.glyph_index('-'),
            _ => None,
        })
    }

    #[must_use]
    pub fn advance(&self, glyph: u16) -> i64 {
        i64::from(self.face.glyph_hor_advance(GlyphId(glyph)).unwrap_or(0))
    }

    /// Unhinted outline at `scale`, shifted so the origin sits at x = 0 as FreeType shifts it.
    #[must_use]
    pub fn outline(&self, glyph: u16, scale: FixedPoint) -> Outline {
        let id = GlyphId(glyph);
        let mut outline = match &self.truetype {
            Some(truetype) => truetype.load(glyph, scale),
            None => {
                let mut builder = ScaledOutline {
                    outline: Outline::default(),
                    scale: scale.raw() as f64 / 65_536.0,
                };

                self.face.outline_glyph(id, &mut builder);
                builder.outline.close_contour();

                builder.outline
            }
        };

        if self.truetype.is_some()
            && let (Some(bounds), Some(bearing)) = (
                self.face.glyph_bounding_box(id),
                self.face.glyph_hor_side_bearing(id),
            )
        {
            let origin = scale.scale(i64::from(bounds.x_min) - i64::from(bearing));

            if origin != 0 {
                outline.translate(-origin, 0);
            }
        }

        outline
    }
}
