use text_mesh_core::Color;
use text_mesh_webgl::*;

#[test]
fn should_index_quads_the_way_tmp_mesh_info_does() {
    let quads = 2;

    let indices = Mesh::indices(quads);

    assert_eq!(indices, [0, 1, 2, 2, 3, 0, 4, 5, 6, 6, 7, 4]);
}

#[test]
fn should_interleave_glyph_vertices_before_decoration_vertices() {
    let vertex = |x: f32| Vertex {
        position: [x, 2.0],
        uv: [0.5, 0.25],
        scale: -1.0,
        color: Color::new(1, 2, 3, 4),
    };
    let mesh = Mesh {
        material: MeshMaterial::Sprites,
        glyphs: vec![vertex(1.0)],
        decorations: vec![vertex(3.0)],
    };

    let data = mesh.vertex_data();

    let first: Vec<u8> = [1.0_f32, 2.0, 0.5, 0.25, -1.0]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .chain([1, 2, 3, 4])
        .collect();

    assert_eq!(
        (
            data.len(),
            &data[..Vertex::STRIDE],
            data[Vertex::STRIDE..][..4].to_vec()
        ),
        (
            2 * Vertex::STRIDE,
            first.as_slice(),
            3.0_f32.to_le_bytes().to_vec()
        )
    );
}
