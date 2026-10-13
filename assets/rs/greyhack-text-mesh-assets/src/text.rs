use std::sync::LazyLock;

use text_mesh_core::TextAssetTable;

/// Maps game asset names to rich-text assets: `Resources` fonts and materials, terminal sprites, and default
/// style sheet styles.
pub static TEXT_ASSETS: LazyLock<TextAssetTable> = LazyLock::new(|| {
    crate::bundle::SETUP
        .text_assets()
        .expect("the game's setup names its text assets")
});
