use text_mesh_core::{Color, Style};
use text_mesh_font::{FontSet, Glyph, GlyphMetrics, GlyphRect};
use text_mesh_layout::{
    Element, ElementGeometry, ElementKind, Layout, LayoutGeometry, LineGeometry, Position, Quad,
};
use text_mesh_test_support::layouts::{LineMetrics, single_line};
use text_mesh_test_support::styles::plain;

use crate::mesh::test_support::sdf_font;

pub fn font_set() -> FontSet {
    let font = |name: &'static str, characters: &str| {
        sdf_font(name).with_glyphs(characters.chars().map(|character| Glyph {
            character,
            metrics: GlyphMetrics::default(),
            rect: GlyphRect {
                x: 10,
                y: 20,
                width: 4,
                height: 6,
                atlas: 0,
            },
        }))
    };

    text_mesh_test_support::fonts::font_set(
        vec![font("Primary SDF", "a_"), font("Fallback SDF", "b")],
        &[1],
    )
}

pub const fn style() -> Style {
    plain(Color::new(0, 240, 221, 255), 20.0)
}

pub fn glyph(
    character: char,
    font: usize,
    origin: f32,
    style: Style,
) -> (Element, ElementGeometry) {
    let element = Element {
        kind: ElementKind::Glyph { character, font },
        style,
        line: 0,
        visible: character != ' ',
        origin,
        advance: origin + 10.0,
        baseline: 20.0,
        ascender: 5.0,
        descender: 25.0,
        left: origin,
        top: 8.0,
        right: origin + 10.0,
        bottom: 22.0,
        point_size: 20.0,
        scale: 0.5,
    };
    let geometry = ElementGeometry {
        quad: Quad {
            bottom_left: Position { x: origin, y: 22.0 },
            top_left: Position { x: origin, y: 8.0 },
            top_right: Position {
                x: origin + 10.0,
                y: 8.0,
            },
            bottom_right: Position {
                x: origin + 10.0,
                y: 22.0,
            },
        },
        font,
        base_font: None,
        padding: 1.0,
        style_padding: 0.5,
    };

    (element, geometry)
}

pub fn layout(elements: Vec<(Element, ElementGeometry)>) -> (Layout, LayoutGeometry) {
    let last = elements.len() - 1;
    let layout = single_line(
        elements.iter().map(|(element, _)| *element).collect(),
        30.0,
        LineMetrics {
            ascender: 5.0,
            descender: 25.0,
            baseline: 20.0,
        },
    );
    let geometry = LayoutGeometry {
        elements: elements.iter().map(|(_, geometry)| *geometry).collect(),
        lines: vec![LineGeometry {
            first_visible: 0,
            last_visible: last,
        }],
    };

    (layout, geometry)
}
