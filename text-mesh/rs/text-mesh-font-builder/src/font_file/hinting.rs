use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{
    DrawSettings, Engine, HintingInstance, HintingOptions, OutlineGlyphCollection, OutlinePen,
    SmoothMode, Target,
};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::outline::{Outline, PointKind};

/// Hints glyph outlines like FreeType `FT_LOAD_DEFAULT`; fonts without instructions remain unhinted for Unity compatibility.
pub struct HintedFont<'a> {
    glyphs: OutlineGlyphCollection<'a>,
    instance: HintingInstance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HintedGlyph {
    pub outline: Outline,
    pub advance: Option<i64>,
}

#[derive(Default)]
struct OutlineRecorder {
    outline: Outline,
}

impl OutlineRecorder {
    fn point(&mut self, x: f32, y: f32, kind: PointKind) {
        let to_26_6 = |value: f32| (f64::from(value) * 64.0).round() as i64;

        self.outline.push_point(to_26_6(x), to_26_6(y), kind);
    }
}

impl OutlinePen for OutlineRecorder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.outline.close_contour();
        self.point(x, y, PointKind::OnCurve);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.point(x, y, PointKind::OnCurve);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.point(cx0, cy0, PointKind::Quadratic);
        self.point(x, y, PointKind::OnCurve);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.point(cx0, cy0, PointKind::Cubic);
        self.point(cx1, cy1, PointKind::Cubic);
        self.point(x, y, PointKind::OnCurve);
    }

    fn close(&mut self) {
        self.outline.close_contour();
    }
}

impl<'a> HintedFont<'a> {
    #[must_use]
    pub fn new(data: &'a [u8], point_size: u16) -> Option<Self> {
        let font = FontRef::new(data).ok()?;
        let glyphs = font.outline_glyphs();
        let options = HintingOptions {
            engine: Engine::Interpreter,
            target: Target::Smooth {
                mode: SmoothMode::Normal,
                symmetric_rendering: true,
                preserve_linear_metrics: false,
            },
        };
        let instance = HintingInstance::new(
            &glyphs,
            Size::new(f32::from(point_size)),
            LocationRef::default(),
            options,
        )
        .ok()?;

        Some(Self { glyphs, instance })
    }

    #[must_use]
    pub fn glyph(&self, glyph: u16) -> Option<HintedGlyph> {
        let outline = self.glyphs.get(GlyphId::new(u32::from(glyph)))?;
        let mut recorder = OutlineRecorder::default();
        let metrics = outline
            .draw(DrawSettings::hinted(&self.instance, false), &mut recorder)
            .ok()?;

        recorder.outline.close_contour();

        Some(HintedGlyph {
            outline: recorder.outline,
            advance: metrics
                .advance_width
                .map(|advance| (f64::from(advance) * 64.0).round() as i64),
        })
    }

    /// Top of the hinted outline in 26.6.
    #[must_use]
    pub fn top(&self, glyph: u16) -> Option<i64> {
        self.glyph(glyph)?
            .outline
            .control_box()
            .map(|bounds| bounds.y_max)
    }
}
