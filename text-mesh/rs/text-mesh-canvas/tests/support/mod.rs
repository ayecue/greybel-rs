#![allow(dead_code)]

use std::borrow::Cow;

use text_mesh_core::{Color, Style};
use text_mesh_font::{FaceInfo, FontAsset, FontSet, FontStyleSettings};
use text_mesh_layout::{Element, ElementKind, Layout};
use text_mesh_test_support::layouts::{LineMetrics, single_line};
use text_mesh_test_support::styles::plain;

pub fn fonts() -> FontSet {
    text_mesh_test_support::fonts::font_set(
        vec![FontAsset {
            name: Cow::Borrowed("Mono SDF"),
            face: FaceInfo {
                point_size: 40.0,
                scale: 1.0,
                underline_offset: -4.0,
                underline_thickness: 2.0,
                strikethrough_offset: 10.0,
                strikethrough_thickness: 2.0,
                ..FaceInfo::default()
            },
            style: FontStyleSettings {
                italic_style: 35,
                ..FontStyleSettings::default()
            },
            ..FontAsset::default()
        }],
        &[],
    )
}

pub const fn style() -> Style {
    plain(Color::WHITE, 20.0)
}

pub fn glyph(character: char, origin: f32, style: Style) -> Element {
    Element {
        kind: ElementKind::Glyph { character, font: 0 },
        style,
        line: 0,
        visible: character != ' ',
        origin,
        advance: origin + 12.0,
        baseline: 18.0,
        ascender: 2.0,
        descender: 22.0,
        left: origin + 1.0,
        top: 6.0,
        right: origin + 11.0,
        bottom: 18.0,
        point_size: 20.0,
        scale: 0.5,
    }
}

pub fn sprite(origin: f32) -> Element {
    Element {
        kind: ElementKind::Sprite {
            index: 0,
            color: Color::new(255, 0, 0, 255),
        },
        left: origin,
        right: origin + 8.0,
        ..glyph('\u{fffe}', origin, style())
    }
}

pub fn layout(elements: Vec<Element>) -> Layout {
    single_line(
        elements,
        24.0,
        LineMetrics {
            ascender: 2.0,
            descender: 22.0,
            baseline: 18.0,
        },
    )
}
