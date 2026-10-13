use text_mesh_core::Style;

use super::*;

#[path = "support/builder.rs"]
mod support;

#[test]
fn should_put_fallback_glyphs_in_a_fallback_mesh_after_the_primary_mesh() {
    let fonts = support::font_set();
    let sprites = SpriteSheet::default();
    let (layout, geometry) = support::layout(vec![
        support::glyph('b', 1, 0.0, support::style()),
        support::glyph('a', 0, 10.0, support::style()),
    ]);

    let meshes =
        MeshBuilder::new(&fonts, &sprites, Color::WHITE).build(&layout, &geometry, [0.0; 2]);

    let materials: Vec<_> = meshes.iter().map(|mesh| mesh.material).collect();

    assert_eq!(
        materials,
        [
            MeshMaterial::Font { font: 0, atlas: 0 },
            MeshMaterial::Fallback {
                source: 0,
                font: 1,
                atlas: 0
            }
        ]
    );
}

#[test]
fn should_negate_the_scale_of_bold_glyphs() {
    let fonts = support::font_set();
    let sprites = SpriteSheet::default();
    let bold = Style {
        bold: true,
        ..support::style()
    };
    let (layout, geometry) = support::layout(vec![support::glyph('a', 0, 0.0, bold)]);

    let meshes =
        MeshBuilder::new(&fonts, &sprites, Color::WHITE).build(&layout, &geometry, [0.0; 2]);

    let scales: Vec<_> = meshes[0].glyphs.iter().map(|vertex| vertex.scale).collect();

    assert_eq!(scales, [-0.5; 4]);
}

#[test]
fn should_grow_the_atlas_uvs_by_the_material_and_style_paddings() {
    let fonts = support::font_set();
    let sprites = SpriteSheet::default();
    let (layout, geometry) = support::layout(vec![support::glyph('a', 0, 0.0, support::style())]);

    let meshes =
        MeshBuilder::new(&fonts, &sprites, Color::WHITE).build(&layout, &geometry, [0.0; 2]);

    let uvs: Vec<_> = meshes[0].glyphs.iter().map(|vertex| vertex.uv).collect();

    assert_eq!(
        uvs,
        [
            [0.085, 0.185],
            [0.085, 0.275],
            [0.155, 0.275],
            [0.155, 0.185]
        ]
    );
}

#[test]
fn should_offset_the_quads_and_cap_the_alpha_by_the_font_color() {
    let fonts = support::font_set();
    let sprites = SpriteSheet::default();
    let (layout, geometry) = support::layout(vec![support::glyph('a', 0, 0.0, support::style())]);
    let font_color = Color::new(255, 255, 255, 128);

    let meshes =
        MeshBuilder::new(&fonts, &sprites, font_color).build(&layout, &geometry, [10.0, 5.0]);

    let vertex = meshes[0].glyphs[0];

    assert_eq!(
        (vertex.position, vertex.color),
        ([10.0, 27.0], Color::new(0, 240, 221, 128))
    );
}

#[test]
fn should_leave_out_invisible_characters() {
    let fonts = support::font_set();
    let sprites = SpriteSheet::default();
    let (layout, geometry) = support::layout(vec![
        support::glyph('a', 0, 0.0, support::style()),
        support::glyph(' ', 0, 10.0, support::style()),
    ]);

    let meshes =
        MeshBuilder::new(&fonts, &sprites, Color::WHITE).build(&layout, &geometry, [0.0; 2]);

    assert_eq!(meshes.iter().map(Mesh::vertex_count).sum::<usize>(), 4);
}
