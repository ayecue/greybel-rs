use text_mesh_core::{Color, Style};
use text_mesh_font::FontSet;
use text_mesh_test_support::styles::plain;

use crate::text_layout::test_support::primary_font;

pub const fn style() -> Style {
    plain(Color::WHITE, 16.5)
}

pub fn font_set() -> FontSet {
    text_mesh_test_support::fonts::font_set(vec![primary_font()], &[])
}
