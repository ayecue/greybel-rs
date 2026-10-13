//! Helpers the mesh tests share.

use std::borrow::Cow;

use text_mesh_core::Color;
use text_mesh_font::{FaceInfo, FontAsset, FontAtlas, SdfMaterial};
use text_mesh_test_support::styles::plain;

use crate::mesh::character::Character;
use crate::mesh::point::Point;

/// The character `code` in plain white 20 pt text, at the origin with no extent.
pub fn character(code: char) -> Character {
    Character {
        code: u32::from(code),
        style: plain(Color::WHITE, 20.0),
        line: 0,
        scale: 1.0,
        point_size: 20.0,
        bottom_left: Point::default(),
        top_right: Point::default(),
        baseline: 0.0,
        ascender: 0.0,
        descender: 0.0,
        font: 0,
    }
}

/// A 40 pt SDF font `name` at scale 1 with a gradient scale of 6 over a 100 × 100 atlas, without glyphs.
pub fn sdf_font(name: &'static str) -> FontAsset {
    FontAsset {
        name: Cow::Borrowed(name),
        face: FaceInfo {
            point_size: 40.0,
            scale: 1.0,
            ..FaceInfo::default()
        },
        material: Some(SdfMaterial {
            gradient_scale: 6.0,
            ..SdfMaterial::default()
        }),
        atlas: Some(FontAtlas {
            width: 100,
            height: 100,
            ..FontAtlas::default()
        }),
        ..FontAsset::default()
    }
}
