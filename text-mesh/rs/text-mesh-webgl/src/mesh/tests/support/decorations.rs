use text_mesh_core::{Color, Style};
use text_mesh_font::{FontSet, Glyph, GlyphMetrics, GlyphRect};
use text_mesh_test_support::styles::plain;

use crate::mesh::character::Character;
use crate::mesh::decorations::{DecorationManager, LineEnds};
use crate::mesh::point::Point;
use crate::mesh::{Vertex, test_support};

/// A primary font with an underline and strikethrough at known offsets and a material padding of 1.25.
pub fn font_set() -> FontSet {
    let mut font = test_support::sdf_font("Primary SDF");

    font.face.underline_offset = -3.0;
    font.face.underline_thickness = 2.0;
    font.face.strikethrough_offset = 8.0;

    let font = font.with_glyphs([Glyph {
        character: '_',
        metrics: GlyphMetrics {
            width: 10.0,
            ..GlyphMetrics::default()
        },
        rect: GlyphRect {
            x: 10,
            y: 20,
            width: 10,
            height: 3,
            atlas: 0,
        },
    }]);

    text_mesh_test_support::fonts::font_set(vec![font], &[])
}

pub const fn style() -> Style {
    plain(Color::WHITE, 20.0)
}

/// A 20 px wide character starting at `left`, its baseline at y = -20, scale 1.
pub fn character(code: char, left: f32, style: Style) -> Character {
    Character {
        style,
        bottom_left: Point::new(left, -28.0),
        top_right: Point::new(left + 20.0, -2.0),
        baseline: -20.0,
        ascender: -4.0,
        descender: -26.0,
        ..test_support::character(code)
    }
}

/// The white decoration vertices of `characters`, all on one line, in `fonts`.
pub fn decoration_vertices(fonts: &FontSet, characters: &[Character]) -> Vec<Vertex> {
    let lines = [LineEnds {
        last: characters.len() - 1,
        last_visible: characters.len() - 1,
    }];
    let mut decorations = DecorationManager::new(fonts, Color::WHITE, characters, &lines, [0.0; 2]);

    for index in 0..characters.len() {
        decorations.character(index, 1.0);
    }

    decorations.into_vertices()
}
