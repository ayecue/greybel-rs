mod alignment;
mod cursor;
mod generator;
mod line_break;
mod metrics;
mod preferred_height;
mod records;
mod source;
mod tags;
mod unicode;

#[cfg(test)]
#[path = "tests/support/shared.rs"]
mod test_support;

use text_mesh_core::{Text, TextAssetTable};
use text_mesh_font::FontSet;

use crate::layout::{Layout, LayoutGeometry};
use crate::settings::TextSettings;
use crate::text_layout::alignment::Aligner;
use crate::text_layout::generator::Generator;
use crate::text_layout::preferred_height::PreferredHeight;
use crate::text_layout::source::Source;

#[derive(Debug, Clone, Copy)]
pub struct TextLayout<'a> {
    fonts: &'a FontSet,
    assets: &'a TextAssetTable,
    settings: &'a TextSettings,
}

impl<'a> TextLayout<'a> {
    #[must_use]
    pub const fn new(
        fonts: &'a FontSet,
        assets: &'a TextAssetTable,
        settings: &'a TextSettings,
    ) -> Self {
        Self {
            fonts,
            assets,
            settings,
        }
    }

    #[must_use]
    pub fn preferred_height(&self, text: &Text, width: f32) -> f32 {
        let source = self.source(text);

        self.measure_height(&source, width)
    }

    #[must_use]
    pub fn layout(&self, text: &Text, width: f32, height: f32) -> Layout {
        self.layout_with_geometry(text, width, height).0
    }

    /// Returns [`Self::layout`] with the per-element and per-line data required by TMP mesh generation.
    #[must_use]
    pub fn layout_with_geometry(
        &self,
        text: &Text,
        width: f32,
        height: f32,
    ) -> (Layout, LayoutGeometry) {
        let source = self.source(text);
        let preferred_height = self.measure_height(&source, width);

        self.arrange(&source, width, height, preferred_height)
    }

    pub(crate) fn layout_with_min_height(
        &self,
        text: &Text,
        width: f32,
        min_height: f32,
    ) -> (Layout, LayoutGeometry) {
        let source = self.source(text);
        let preferred_height = self.measure_height(&source, width);

        self.arrange(
            &source,
            width,
            preferred_height.max(min_height),
            preferred_height,
        )
    }

    fn source(&self, text: &Text) -> Source {
        Source::new(text, self.fonts, self.assets, self.settings.font_size)
    }

    fn margin_width(&self, width: f32) -> f32 {
        width - self.settings.margin.left - self.settings.margin.right
    }

    fn measure_height(&self, source: &Source, width: f32) -> f32 {
        PreferredHeight::new(self.fonts, self.settings, source, self.margin_width(width)).compute()
    }

    fn arrange(
        &self,
        source: &Source,
        width: f32,
        height: f32,
        preferred_height: f32,
    ) -> (Layout, LayoutGeometry) {
        if source.elements.is_empty() {
            let layout = Layout {
                width,
                height,
                preferred_height,
                ..Layout::default()
            };

            return (layout, LayoutGeometry::default());
        }

        let generated_text =
            Generator::new(self.fonts, self.settings, source, self.margin_width(width)).generate();

        Aligner::new(self.settings, source, generated_text, width, height).layout(preferred_height)
    }
}

/// Evaluates TMP `value += addend` with Mono semantics: the float addend expression uses double precision
/// and the sum rounds to float once instead of rounding each single-precision step.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn mono_sum(value: f32, addend: f64) -> f32 {
    (f64::from(value) + addend) as f32
}
