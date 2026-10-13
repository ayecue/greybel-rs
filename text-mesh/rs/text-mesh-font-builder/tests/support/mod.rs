//! Unity's bakes of the test setup's fonts (text-mesh-test-support): its export for the fonts the setup bakes, the
//! recordings in `tests/fixtures/unity` for the others.

#![allow(dead_code)]

pub mod comparison;
pub mod determinism;
pub mod kerning;
pub mod render_modes;
pub mod unity;

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use text_mesh_font::RenderMode;
use text_mesh_font_builder::{BuiltFont, FontBuildSettings, FontBuilder};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    pub name: String,
    pub source: String,
    pub point_size: u16,
    pub padding: u16,
    pub atlas_width: u16,
    pub atlas_height: u16,
    pub render_mode: u32,
    pub face: RecordedFace,
    pub glyphs: Vec<RecordedGlyph>,
    pub atlases: Vec<String>,
    /// The folder of the `atlases`.
    #[serde(skip)]
    pub atlas_folder: PathBuf,
}

/// A font of the test setup's export (`text-mesh.json`), baked by the setup with the settings of a recording.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportedFont {
    name: String,
    source: String,
    point_size: f32,
    padding: u16,
    atlas_width: u16,
    atlas_height: u16,
    render_mode: u32,
    face: Value,
    characters: Vec<RecordedGlyph>,
    atlases: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedFace {
    pub point_size: f32,
    pub scale: f32,
    pub line_height: f32,
    pub ascent_line: f32,
    pub cap_line: f32,
    pub mean_line: f32,
    pub baseline: f32,
    pub descent_line: f32,
    pub superscript_offset: f32,
    pub superscript_size: f32,
    pub subscript_offset: f32,
    pub subscript_size: f32,
    pub underline_offset: f32,
    pub underline_thickness: f32,
    pub strikethrough_offset: f32,
    pub strikethrough_thickness: f32,
    pub tab_width: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedGlyph {
    pub unicode: u32,
    pub width: f32,
    pub height: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance: f32,
    pub x: u16,
    pub y: u16,
    pub rect_width: u16,
    pub rect_height: u16,
    pub atlas: u8,
}

/// An 8-bit image decoded from a recorded PNG, rows from the top.
#[derive(Debug, Clone)]
pub struct RecordedImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl RecordedImage {
    pub fn pixel(&self, x: i64, row: i64) -> u8 {
        if x < 0 || row < 0 || x as usize >= self.width || row as usize >= self.height {
            return 0;
        }

        self.pixels[row as usize * self.width + x as usize]
    }
}

/// The font `data` built with `settings`.
///
/// # Panics
/// When the font doesn't parse or build.
pub fn build_font(data: &[u8], settings: FontBuildSettings) -> BuiltFont {
    FontBuilder::new(data, settings)
        .expect("the font parses")
        .build()
        .expect("the font builds")
}

pub fn fixtures_dir() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures/unity")
}

/// What Unity's FontEngine bakes from each font of the test setup, by name: the fonts the setup bakes from its export
/// (`baked_fonts`), the others from their recordings (`recorded_fonts`, refreshed first with `REFRESH_REFERENCE` set).
pub fn recordings() -> Vec<Recording> {
    let mut recordings = baked_fonts();

    recordings.extend(recorded_fonts());
    recordings.sort_by(|first, second| first.name.cmp(&second.name));

    recordings
}

/// The fonts the test setup bakes (no `atlases` of their own in `setup.json`), as its export has them.
pub fn baked_fonts() -> Vec<Recording> {
    let folder = text_mesh_test_support::folder();
    let setup = read_json(&text_mesh_test_support::setup_file());
    let baked: Vec<&str> = setup["fonts"]
        .as_array()
        .expect("the setup's fonts")
        .iter()
        .filter(|font| font.get("atlases").is_none())
        .filter_map(|font| font["name"].as_str())
        .collect();
    let export = read_json(&folder.join("text-mesh.json"));
    let fonts: Vec<ExportedFont> =
        serde_json::from_value(export["fonts"].clone()).expect("the export's fonts");

    fonts
        .into_iter()
        .filter(|font| baked.contains(&font.name.as_str()))
        .map(|font| Recording {
            name: font.name,
            source: font.source,
            point_size: font.point_size as u16,
            padding: font.padding,
            atlas_width: font.atlas_width,
            atlas_height: font.atlas_height,
            render_mode: font.render_mode,
            face: serde_json::from_value(camel_case_keys(font.face)).expect("the export's face"),
            glyphs: font.characters,
            atlases: font.atlases,
            atlas_folder: folder.join("atlases"),
        })
        .collect()
}

/// Unity's recordings of the test setup's other fonts: the given and bundled ones, baked again from their sources.
pub fn recorded_fonts() -> Vec<Recording> {
    text_mesh_test_support::refresh_reference("fonts", |reference, setup| {
        reference.record_fonts(setup, &fixtures_dir())
    });

    let mut paths: Vec<PathBuf> = std::fs::read_dir(fixtures_dir())
        .expect("the fixtures are listed")
        .map(|entry| entry.expect("a fixture entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();

    paths.sort();

    paths
        .iter()
        .map(|path| {
            let mut recording: Recording = serde_json::from_value(read_json(path))
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));

            recording.atlas_folder = fixtures_dir();

            recording
        })
        .collect()
}

fn read_json(path: &Path) -> Value {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));

    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// `value` with the first letter of each key in lower case: the export's `face` names its fields as TMP's `FaceInfo`.
fn camel_case_keys(value: Value) -> Value {
    let Value::Object(fields) = value else {
        return value;
    };
    let fields: serde_json::Map<String, Value> = fields
        .into_iter()
        .map(|(key, value)| {
            let mut characters = key.chars();
            let key = characters
                .next()
                .map(|first| first.to_lowercase().chain(characters).collect::<String>())
                .unwrap_or_default();

            (key, value)
        })
        .collect();

    Value::Object(fields)
}

pub fn decode_png(path: &Path) -> RecordedImage {
    let file =
        std::fs::File::open(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().expect("a PNG header");
    let mut buffer = vec![0; reader.output_buffer_size().expect("a PNG size")];
    let info = reader.next_frame(&mut buffer).expect("a PNG frame");
    let samples = info.color_type.samples();

    RecordedImage {
        width: info.width as usize,
        height: info.height as usize,
        pixels: buffer[..info.buffer_size()]
            .chunks(samples)
            .map(|pixel| pixel[0])
            .collect(),
    }
}

impl Recording {
    pub fn render_mode(&self) -> RenderMode {
        RenderMode::from_unity(self.render_mode).expect("a known render mode")
    }

    /// The recording's settings, with its characters in the order the setup lists for them (ascending).
    pub fn settings(&self) -> FontBuildSettings {
        let characters = self
            .glyphs
            .iter()
            .filter_map(|glyph| char::from_u32(glyph.unicode));

        FontBuildSettings::new(self.name.clone(), characters)
            .with_sampling_size(self.point_size, self.padding)
            .with_atlas(self.atlas_width, self.atlas_height, self.render_mode())
    }

    pub fn build(&self) -> BuiltFont {
        build_font(
            &text_mesh_test_support::font_file(&self.source),
            self.settings(),
        )
    }

    pub fn atlas_images(&self) -> Vec<RecordedImage> {
        self.atlases
            .iter()
            .map(|file| decode_png(&self.atlas_folder.join(file)))
            .collect()
    }
}
