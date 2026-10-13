//! Unity/TMP mesh references for shared text-mesh cases using Grey Hack's setup.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use greyhack_text_mesh_assets::{
    AtlasFile, FontFile, TERMINAL_TEXT, TERMINAL_THEME, TEXT_ASSETS, TerminalText,
};
use text_mesh_font::FontSet;
use text_mesh_layout::{LayoutGeometry, TerminalItem};
use text_mesh_test_support::meshes::{MeshReference, Terminal};
use text_mesh_test_support::reference::KnownDifference;
use text_mesh_webgl::SpriteSheet;

pub const KNOWN_DIFFERENCES: &[KnownDifference] = &[];

/// Builds the shared `MeshReference` using Grey Hack's setup, fonts, original atlases, and sprite sheet, matching the
/// Unity emulator's inputs.
pub fn reference() -> MeshReference<'static> {
    MeshReference {
        fixtures: Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        setup: game_setup,
        terminal: terminal(),
        fonts: fonts(),
        sprites: &SPRITES,
    }
}

/// Writes the game setup into a temporary `text-mesh.json` layout with `fonts/`, `atlases/`, and `sprites/`, then
/// returns the setup path.
pub fn game_setup() -> PathBuf {
    let folder = std::env::temp_dir().join(format!("greyhack-setup-{}", std::process::id()));
    let write = |path: &Path, data: &[u8]| {
        std::fs::create_dir_all(path.parent().expect("a file in a folder"))
            .expect("the folder is made");
        std::fs::write(path, data).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    };

    for font in FontFile::all() {
        write(&folder.join("fonts").join(font.file_name), font.data);
    }

    for atlas in AtlasFile::all() {
        write(
            &folder
                .join("atlases")
                .join(format!("{}.{}.png", atlas.font, atlas.index)),
            &png_file(
                atlas.width,
                atlas.height,
                png::ColorType::Grayscale,
                atlas.alpha,
            ),
        );
    }

    let sheet = TERMINAL_TEXT.sprite_sheet;

    write(
        &folder
            .join("sprites")
            .join(format!("{}.png", TERMINAL_TEXT.font_set.sprites.name)),
        &png_file(sheet.width, sheet.height, png::ColorType::Rgba, sheet.rgba),
    );

    let setup = folder.join("text-mesh.json");

    write(&setup, TerminalText::game_setup().as_bytes());

    setup
}

/// Encodes top-down pixel rows as an 8-bit PNG in the format expected by the Unity reference project.
fn png_file(width: u16, height: u16, color: png::ColorType, pixels: &[u8]) -> Vec<u8> {
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, u32::from(width), u32::from(height));

    encoder.set_color(color);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().expect("a PNG header is written");

    writer
        .write_image_data(pixels)
        .expect("the pixels are written");
    writer.finish().expect("the PNG is finished");

    file
}

/// Returns the game terminal configuration excluding its fonts.
pub fn terminal() -> Terminal<'static> {
    Terminal {
        assets: &TEXT_ASSETS,
        settings: &TERMINAL_TEXT.settings,
        component: TERMINAL_THEME.component(&TEXT_ASSETS),
        frame: TERMINAL_TEXT.frame,
        text: TERMINAL_THEME.text,
    }
}

/// Returns the game font set with the original atlases used by the Unity emulator.
pub fn fonts() -> &'static FontSet {
    &TERMINAL_TEXT.font_set
}

/// Returns the game sprite sheet used by the Unity emulator.
pub fn sprite_sheet() -> SpriteSheet {
    SpriteSheet::from_asset(&TERMINAL_TEXT.font_set.sprites).expect("the game's sprite sheet")
}

static SPRITES: LazyLock<SpriteSheet> = LazyLock::new(sprite_sheet);

/// Lays out each source line in the game terminal with `fonts` and `width`, returning items and geometry.
pub fn lay_out(fonts: &FontSet, source: &str, width: f32) -> Vec<(TerminalItem, LayoutGeometry)> {
    terminal().lay_out(fonts, source, width)
}
