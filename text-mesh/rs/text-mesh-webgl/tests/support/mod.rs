#![allow(dead_code)]

pub mod uniforms;

use std::path::Path;
use std::sync::LazyLock;

use text_mesh_test_support::meshes::MeshReference;
use text_mesh_test_support::reference::KnownDifference;
use text_mesh_webgl::SpriteSheet;

pub const KNOWN_DIFFERENCES: &[KnownDifference] = &[];

static SPRITES: LazyLock<SpriteSheet> = LazyLock::new(|| {
    SpriteSheet::from_asset(&text_mesh_test_support::setup().fonts.sprites)
        .expect("the test setup's sprite sheet")
});

/// The meshes TMP drew every case with through the test setup, with its fonts and the atlases Unity baked, which the
/// reference project draws with too.
pub fn reference() -> MeshReference<'static> {
    let setup = text_mesh_test_support::setup();

    MeshReference {
        fixtures: Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        setup: text_mesh_test_support::setup_file,
        terminal: setup.terminal(),
        fonts: &setup.fonts,
        sprites: &SPRITES,
    }
}
