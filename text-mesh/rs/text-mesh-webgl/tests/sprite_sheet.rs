use std::borrow::Cow;

use text_mesh_font::GlyphRect;
use text_mesh_webgl::*;

#[test]
fn should_map_a_sprite_rect_to_uvs_from_the_sheet_bottom() {
    let sheet = SpriteSheet {
        width: 256,
        height: 128,
        rects: Cow::Owned(vec![GlyphRect {
            x: 64,
            y: 32,
            width: 32,
            height: 64,
            atlas: 0,
        }]),
    };

    let uvs = sheet.uvs(0);

    assert_eq!(
        uvs,
        Some([[0.25, 0.25], [0.25, 0.75], [0.375, 0.75], [0.375, 0.25]])
    );
}

#[test]
fn should_have_no_uvs_for_an_unknown_sprite() {
    let sheet = SpriteSheet::default();

    let uvs = sheet.uvs(3);

    assert_eq!(uvs, None);
}
