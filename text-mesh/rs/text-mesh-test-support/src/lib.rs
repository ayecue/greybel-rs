//! Shared text-mesh fixtures with Unity-exported setup and atlas data.
//! `REFRESH_REFERENCE` regenerates Unity fixtures before tests.

pub mod fonts;
pub mod layouts;
#[cfg(feature = "meshes")]
pub mod meshes;
pub mod reference;
pub mod styles;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, PoisonError};

use ::text_mesh_core::{CellMetricContainer, Color, TextAssetTable, TextComponent};
use ::text_mesh_font::FontSet;
use ::text_mesh_layout::{TerminalFrame, TerminalLayout, TextLayout, TextSettings};
use ::text_mesh_setup::Setup;
pub use ::text_mesh_unity::Reference;

/// Terminal setup the reference project exported, read into text-mesh's types.
#[derive(Debug, Clone)]
pub struct TestSetup {
    pub fonts: FontSet,
    pub settings: TextSettings,
    pub frame: TerminalFrame,
    pub assets: TextAssetTable,
    pub text: Color,
    pub background: Color,
    /// Font names and source files (`assets/fonts`) for file-backed fonts.
    pub sources: Vec<(String, String)>,
}

static SETUP: LazyLock<TestSetup> = LazyLock::new(|| {
    refresh_reference("setup", |reference, setup| {
        reference.export(setup, &folder())
    });

    let path = folder().join("text-mesh.json");
    let json = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}; refresh it with REFRESH_REFERENCE=1 cargo test -p text-mesh-test-support",
            path.display()
        )
    });
    let setup = Setup::read(&json).expect("text-mesh.json is the exported setup");
    let (text, background) = setup.colors().expect("the setup has colors");

    TestSetup {
        fonts: setup
            .font_set()
            .expect("the exported setup gives every font"),
        settings: setup.text_settings(),
        frame: setup.frame().expect("the setup has a frame"),
        assets: setup
            .text_assets()
            .expect("the exported setup names its text assets"),
        text,
        background,
        sources: setup
            .font_sources()
            .into_iter()
            .map(|(name, source)| (name.to_owned(), source.to_owned()))
            .collect(),
    }
});

/// Exported terminal setup.
#[must_use]
pub fn setup() -> &'static TestSetup {
    &SETUP
}

impl TestSetup {
    /// Terminal line's text: the theme's color at the settings' size.
    #[must_use]
    pub const fn component(&self) -> TextComponent<'_> {
        TextComponent::new(self.text, self.settings.font_size, &self.assets)
    }

    #[must_use]
    pub const fn text_layout(&self) -> TextLayout<'_> {
        TextLayout::new(&self.fonts, &self.assets, &self.settings)
    }

    #[must_use]
    pub const fn terminal_layout(&self) -> TerminalLayout<'_> {
        TerminalLayout::new(self.text_layout(), self.component(), self.frame)
    }

    /// Terminal font's cell, relative to the font size: its space_advance's advance and its line height.
    #[must_use]
    pub fn cells(&self) -> CellMetricContainer {
        let font = &self.fonts.fonts[self.fonts.primary];
        let space_advance = font
            .glyph(' ')
            .map_or(font.face.point_size / 2.0, |metrics| metrics.advance);

        CellMetricContainer {
            advance: space_advance / font.face.point_size,
            line_height: font.face.line_height / font.face.point_size,
        }
    }
}

/// Folder of the fonts, the sprite sheet, the setup and its export.
#[must_use]
pub fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// Setup the reference project bakes (`assets/setup.json`).
#[must_use]
pub fn setup_file() -> PathBuf {
    folder().join("setup.json")
}

/// Font file of `assets/fonts`, by its file name.
///
/// # Panics
/// When there is no such font.
#[must_use]
pub fn font_file(source: &str) -> Vec<u8> {
    read(&folder().join("fonts").join(source))
}

/// Rich text cases, one terminal line each, whose layout and meshes the text-mesh crates compare with TMP's: every
/// tag, wrapping, alignment, fallback fonts and sprites.
#[must_use]
pub fn cases() -> String {
    String::from_utf8(read(&folder().join("cases.txt"))).expect("cases.txt is UTF-8")
}

/// Sprite sheet, an RGBA PNG.
#[must_use]
pub fn sprite_sheet() -> Vec<u8> {
    read(&folder().join("sprites/shapes.png"))
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Recordings made by this process; Unity runs one at a time.
static RECORDINGS: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);

/// Invokes `record` for each `name` before fixture loading when `REFRESH_REFERENCE` is set.
/// Requires a shared test process; run `cargo test`, not cargo-nextest.
///
/// # Panics
/// Panics if recording fails or runs under cargo-nextest.
pub fn refresh_reference(
    name: &'static str,
    record: impl FnOnce(&Reference, &Path) -> Result<(), text_mesh_unity::ReferenceError>,
) {
    refresh_reference_over(name, setup_file, record);
}

/// Invokes `record` for each `name` using the setup file produced by `setup` when `REFRESH_REFERENCE` is set.
///
/// # Panics
/// Panics if recording fails or runs under cargo-nextest.
pub fn refresh_reference_over(
    name: &'static str,
    setup: impl FnOnce() -> PathBuf,
    record: impl FnOnce(&Reference, &Path) -> Result<(), text_mesh_unity::ReferenceError>,
) {
    if std::env::var_os("REFRESH_REFERENCE").is_none() {
        return;
    }

    assert!(
        std::env::var_os("NEXTEST_EXECUTION_MODE").is_none_or(|mode| mode != "process-per-test"),
        "REFRESH_REFERENCE refreshes once per process: refresh with `cargo test`, not cargo-nextest, which runs every test in \
         a process of its own"
    );

    let mut recordings = RECORDINGS.lock().unwrap_or_else(PoisonError::into_inner);

    if !recordings.get_or_insert_with(HashSet::new).insert(name) {
        return;
    }

    let reference = Reference::locate().expect("the reference project is found");

    record(&reference, &setup()).unwrap_or_else(|error| panic!("recording {name} failed: {error}"));
}
