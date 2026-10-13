use text_mesh_font::{FaceInfo, FontAsset, FontSet};

use crate::text_layout::tags::TagContext;
use crate::text_layout::test_support::primary_font;

pub fn font_set() -> FontSet {
    let primary = primary_font();
    let secondary = FontAsset {
        name: "Secondary SDF".into(),
        face: FaceInfo {
            point_size: 80.0,
            scale: 1.0,
            line_height: 100.0,
            ..FaceInfo::default()
        },
        ..primary.clone()
    };

    text_mesh_test_support::fonts::font_set(vec![primary, secondary], &[])
}

/// The context of a tag naming font `tag_font` in `fonts`, the primary font current, over `margin_width`.
pub fn tag_context(fonts: &FontSet, margin_width: f32, tag_font: Option<usize>) -> TagContext<'_> {
    TagContext {
        fonts,
        margin_width,
        current_font: 0,
        tag_font,
    }
}
