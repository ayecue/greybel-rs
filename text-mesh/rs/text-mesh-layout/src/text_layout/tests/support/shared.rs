use text_mesh_font::{FaceInfo, FontAsset};
use text_mesh_test_support::fonts::font;

/// The 40 pt font "Primary SDF" at scale 1 with a 50 px line height, of `a`, `b` and space, each 20 px wide.
pub fn primary_font() -> FontAsset {
    FontAsset {
        face: FaceInfo {
            point_size: 40.0,
            scale: 1.0,
            line_height: 50.0,
            ..FaceInfo::default()
        },
        ..font("Primary SDF", "ab ", 20.0, &[])
    }
}
