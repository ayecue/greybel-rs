use super::*;

#[path = "support/decorations.rs"]
mod support;

#[test]
fn should_draw_an_underline_as_three_segments_below_the_run() {
    let fonts = support::font_set();
    let underlined = text_mesh_core::Style {
        underline: Some(Color::WHITE),
        ..support::style()
    };
    let characters = [
        support::character('a', 0.0, underlined),
        support::character('b', 20.0, underlined),
    ];

    let vertices = support::decoration_vertices(&fonts, &characters);

    let positions: Vec<_> = vertices.iter().map(|vertex| vertex.position).collect();

    assert_eq!(
        positions,
        [
            [0.0, 26.25],
            [0.0, 21.75],
            [5.0, 21.75],
            [5.0, 26.25],
            [5.0, 26.25],
            [5.0, 21.75],
            [35.0, 21.75],
            [35.0, 26.25],
            [35.0, 26.25],
            [35.0, 21.75],
            [40.0, 21.75],
            [40.0, 26.25]
        ]
    );
}

#[test]
fn should_draw_a_strikethrough_at_the_fonts_strikethrough_offset() {
    let fonts = support::font_set();
    let struck = text_mesh_core::Style {
        strikethrough: Some(Color::WHITE),
        ..support::style()
    };
    let characters = [support::character('a', 0.0, struck)];

    let vertices = support::decoration_vertices(&fonts, &characters);

    assert_eq!(
        (vertices.len(), vertices[0].position, vertices[11].position),
        (12, [0.0, 15.25], [20.0, 15.25])
    );
}

#[test]
fn should_highlight_from_the_descender_to_the_ascender_with_the_underline_glyphs_centre() {
    let fonts = support::font_set();
    let marked = text_mesh_core::Style {
        highlight: Some(Color::new(255, 0, 0, 128)),
        ..support::style()
    };
    let characters = [
        support::character('a', 0.0, marked),
        support::character('b', 20.0, marked),
    ];

    let vertices = support::decoration_vertices(&fonts, &characters);

    let positions: Vec<_> = vertices.iter().map(|vertex| vertex.position).collect();

    assert_eq!(
        (positions, vertices[0].uv, vertices[0].scale),
        (
            vec![[0.0, 26.0], [0.0, 4.0], [40.0, 4.0], [40.0, 26.0]],
            [0.15, 0.215],
            1.0
        )
    );
}

#[test]
fn should_split_a_highlight_halfway_where_its_color_changes() {
    let fonts = support::font_set();
    let red = text_mesh_core::Style {
        highlight: Some(Color::new(255, 0, 0, 128)),
        ..support::style()
    };
    let blue = text_mesh_core::Style {
        highlight: Some(Color::new(0, 0, 255, 128)),
        ..support::style()
    };
    let characters = [
        support::character('a', 0.0, red),
        support::character('b', 30.0, blue),
    ];

    let vertices = support::decoration_vertices(&fonts, &characters);

    let edges: Vec<_> = vertices
        .chunks(4)
        .map(|quad| (quad[0].position[0], quad[2].position[0]))
        .collect();

    assert_eq!(edges, [(0.0, 25.0), (25.0, 50.0)]);
}
