use std::sync::OnceLock;

use text_mesh_font::{FaceInfo, Glyph, GlyphRect};
use text_mesh_font_builder::BuiltFont;

use super::{RecordedFace, RecordedGlyph, RecordedImage, Recording, recordings};

/// A recording next to what the builder makes of the same settings.
pub struct BuiltRecording {
    pub recording: Recording,
    pub font: BuiltFont,
    pub images: Vec<RecordedImage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MetricDifference {
    pub width: f32,
    pub height: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance: f32,
    pub glyphs: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PixelDifference {
    pub mean: f64,
    pub max: u8,
    pub pixels: usize,
    pub glyphs: usize,
}

pub fn built_recordings() -> &'static [BuiltRecording] {
    static BUILT: OnceLock<Vec<BuiltRecording>> = OnceLock::new();

    BUILT.get_or_init(|| {
        recordings()
            .into_iter()
            .map(|recording| {
                let font = recording.build();
                let images = recording.atlas_images();

                BuiltRecording {
                    recording,
                    font,
                    images,
                }
            })
            .collect()
    })
}

impl RecordedFace {
    pub fn fields(&self) -> [(&'static str, f32); 17] {
        [
            ("point_size", self.point_size),
            ("scale", self.scale),
            ("line_height", self.line_height),
            ("ascent_line", self.ascent_line),
            ("cap_line", self.cap_line),
            ("mean_line", self.mean_line),
            ("baseline", self.baseline),
            ("descent_line", self.descent_line),
            ("superscript_offset", self.superscript_offset),
            ("superscript_size", self.superscript_size),
            ("subscript_offset", self.subscript_offset),
            ("subscript_size", self.subscript_size),
            ("underline_offset", self.underline_offset),
            ("underline_thickness", self.underline_thickness),
            ("strikethrough_offset", self.strikethrough_offset),
            ("strikethrough_thickness", self.strikethrough_thickness),
            ("tab_width", self.tab_width),
        ]
    }
}

pub struct FaceFields;

impl FaceFields {
    pub fn of(face: &FaceInfo) -> [(&'static str, f32); 17] {
        [
            ("point_size", face.point_size),
            ("scale", face.scale),
            ("line_height", face.line_height),
            ("ascent_line", face.ascent_line),
            ("cap_line", face.cap_line),
            ("mean_line", face.mean_line),
            ("baseline", face.baseline),
            ("descent_line", face.descent_line),
            ("superscript_offset", face.superscript_offset),
            ("superscript_size", face.superscript_size),
            ("subscript_offset", face.subscript_offset),
            ("subscript_size", face.subscript_size),
            ("underline_offset", face.underline_offset),
            ("underline_thickness", face.underline_thickness),
            ("strikethrough_offset", face.strikethrough_offset),
            ("strikethrough_thickness", face.strikethrough_thickness),
            ("tab_width", face.tab_width),
        ]
    }
}

impl RecordedGlyph {
    pub fn character(&self) -> char {
        char::from_u32(self.unicode).expect("a recorded character")
    }

    pub fn rect(&self) -> GlyphRect {
        GlyphRect {
            x: self.x,
            y: self.y,
            width: self.rect_width,
            height: self.rect_height,
            atlas: self.atlas,
        }
    }
}

impl BuiltRecording {
    pub fn name(&self) -> &str {
        &self.recording.name
    }

    pub fn glyph(&self, character: char) -> Option<&Glyph> {
        self.font
            .asset
            .glyphs
            .iter()
            .find(|glyph| glyph.character == character)
    }

    /// The face info fields whose `f32` bits differ: name, ours, Unity's.
    pub fn face_differences(&self) -> Vec<(&'static str, f32, f32)> {
        FaceFields::of(&self.font.asset.face)
            .into_iter()
            .zip(self.recording.face.fields())
            .filter(|((_, ours), (_, unity))| ours.to_bits() != unity.to_bits())
            .map(|((name, ours), (_, unity))| (name, ours, unity))
            .collect()
    }

    /// The characters Unity has a glyph for and we do not, or the other
    /// way round.
    pub fn character_differences(&self) -> Vec<char> {
        let recorded: Vec<char> = self
            .recording
            .glyphs
            .iter()
            .map(RecordedGlyph::character)
            .collect();
        let ours: Vec<char> = self
            .font
            .asset
            .glyphs
            .iter()
            .map(|glyph| glyph.character)
            .collect();
        let mut differences: Vec<char> = recorded
            .iter()
            .filter(|character| !ours.contains(character))
            .chain(
                ours.iter()
                    .filter(|character| !recorded.contains(character)),
            )
            .copied()
            .collect();

        differences.sort_unstable();

        differences
    }

    pub fn metric_difference(&self) -> MetricDifference {
        let mut difference = MetricDifference::default();

        for recorded in &self.recording.glyphs {
            let Some(glyph) = self.glyph(recorded.character()) else {
                continue;
            };
            let metrics = glyph.metrics;
            let fields = [
                (&mut difference.width, metrics.width, recorded.width),
                (&mut difference.height, metrics.height, recorded.height),
                (
                    &mut difference.bearing_x,
                    metrics.bearing_x,
                    recorded.bearing_x,
                ),
                (
                    &mut difference.bearing_y,
                    metrics.bearing_y,
                    recorded.bearing_y,
                ),
                (&mut difference.advance, metrics.advance, recorded.advance),
            ];
            let mut differs = false;

            for (max, ours, unity) in fields {
                let delta = (ours - unity).abs();

                differs |= delta > 0.0;
                *max = max.max(delta);
            }

            difference.glyphs += usize::from(differs);
        }

        difference
    }

    /// The characters whose atlas rectangle differs: ours, Unity's.
    pub fn rect_differences(&self) -> Vec<(char, GlyphRect, GlyphRect)> {
        self.recording
            .glyphs
            .iter()
            .filter_map(|recorded| {
                let glyph = self.glyph(recorded.character())?;

                (glyph.rect != recorded.rect())
                    .then(|| (recorded.character(), glyph.rect, recorded.rect()))
            })
            .collect()
    }

    /// Our atlas pixels against Unity's over every glyph's rectangle grown
    /// by the padding, each cropped at our and at Unity's position.
    pub fn pixel_difference(&self) -> PixelDifference {
        let padding = i64::from(self.recording.padding);
        let mut total = 0u64;
        let mut difference = PixelDifference::default();
        let mut seen = Vec::new();

        for recorded in &self.recording.glyphs {
            if recorded.rect_width == 0
                || recorded.rect_height == 0
                || seen.contains(&(recorded.atlas, recorded.x, recorded.y))
            {
                continue;
            }

            seen.push((recorded.atlas, recorded.x, recorded.y));

            let Some(glyph) = self.glyph(recorded.character()) else {
                continue;
            };
            let ours = &self.font.atlases[usize::from(glyph.rect.atlas)];
            let theirs = &self.images[usize::from(recorded.atlas)];
            let (width, height) = (
                i64::from(recorded.rect_width) + 2 * padding,
                i64::from(recorded.rect_height) + 2 * padding,
            );
            let our_top = i64::from(ours.height)
                - i64::from(glyph.rect.y)
                - i64::from(glyph.rect.height)
                - padding;
            let their_top = theirs.height as i64
                - i64::from(recorded.y)
                - i64::from(recorded.rect_height)
                - padding;
            let (our_left, their_left) = (
                i64::from(glyph.rect.x) - padding,
                i64::from(recorded.x) - padding,
            );

            for row in 0..height {
                for column in 0..width {
                    let mine = if our_left + column < 0 || our_top + row < 0 {
                        0
                    } else {
                        ours.pixel((our_left + column) as usize, (our_top + row) as usize)
                    };
                    let unity = theirs.pixel(their_left + column, their_top + row);
                    let delta = mine.abs_diff(unity);

                    total += u64::from(delta);
                    difference.max = difference.max.max(delta);
                    difference.pixels += 1;
                }
            }

            difference.glyphs += 1;
        }

        difference.mean = total as f64 / difference.pixels.max(1) as f64;

        difference
    }

    /// Our atlases against Unity's, every pixel (the glyphs sit at the same
    /// positions).
    pub fn atlas_difference(&self) -> PixelDifference {
        let mut total = 0u64;
        let mut difference = PixelDifference::default();

        for (ours, theirs) in self.font.atlases.iter().zip(&self.images) {
            for row in 0..theirs.height {
                for column in 0..theirs.width {
                    let delta = ours
                        .pixel(column, row)
                        .abs_diff(theirs.pixel(column as i64, row as i64));

                    total += u64::from(delta);
                    difference.max = difference.max.max(delta);
                    difference.pixels += 1;
                }
            }
        }

        difference.mean = total as f64 / difference.pixels.max(1) as f64;

        difference
    }

    pub fn report(&self) -> String {
        let metrics = self.metric_difference();
        let pixels = self.pixel_difference();
        let atlas = self.atlas_difference();

        format!(
            "{}: face differences {:?}; characters {:?}; metrics {:?}; rects differing {} of {}; atlases {} vs {}; glyph pixels mean {:.4} max {} over {} glyphs ({} px); full atlases mean {:.4} max {}",
            self.name(),
            self.face_differences(),
            self.character_differences(),
            metrics,
            self.rect_differences().len(),
            self.recording.glyphs.len(),
            self.font.atlases.len(),
            self.images.len(),
            pixels.mean,
            pixels.max,
            pixels.glyphs,
            pixels.pixels,
            atlas.mean,
            atlas.max,
        )
    }
}
