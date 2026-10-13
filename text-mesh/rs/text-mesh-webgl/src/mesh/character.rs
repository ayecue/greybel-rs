use text_mesh_core::Style;
use text_mesh_layout::{Element, ElementGeometry, ElementKind};
use unicode_general_category::{GeneralCategory, get_general_category};

use crate::mesh::point::Point;

/// Parts of TMP's `TMP_CharacterInfo` the second pass reads, in mesh space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Character {
    pub code: u32,
    pub style: Style,
    pub line: usize,
    pub scale: f32,
    pub point_size: f32,
    pub bottom_left: Point,
    pub top_right: Point,
    pub baseline: f32,
    pub ascender: f32,
    pub descender: f32,
    pub font: usize,
}

impl Character {
    pub(crate) fn new(element: &Element, geometry: &ElementGeometry) -> Self {
        let code = match element.kind {
            ElementKind::Glyph { character, .. } => u32::from(character),
            ElementKind::Sprite { index, .. } => 0xE000 + index,
        };

        Self {
            code,
            style: element.style,
            line: element.line,
            scale: element.scale,
            point_size: element.point_size,
            bottom_left: Point::from_layout(geometry.quad.bottom_left),
            top_right: Point::from_layout(geometry.quad.top_right),
            baseline: -element.baseline,
            ascender: -element.ascender,
            descender: -element.descender,
            font: geometry.font,
        }
    }

    /// Applies .NET `char.IsWhiteSpace` to the UTF-16 code unit stored by TMP.
    pub(crate) fn is_white_space(&self) -> bool {
        self.utf16_char().is_some_and(char::is_whitespace)
    }

    /// Implements .NET `char.IsSeparator` for Unicode categories Zs, Zl, and Zp.
    pub(crate) fn is_separator(&self) -> bool {
        self.utf16_char().is_some_and(|character| {
            matches!(
                get_general_category(character),
                GeneralCategory::SpaceSeparator
                    | GeneralCategory::LineSeparator
                    | GeneralCategory::ParagraphSeparator
            )
        })
    }

    pub(crate) const fn is_line_break(&self) -> bool {
        matches!(self.code, 10 | 11 | 13)
    }

    fn utf16_char(&self) -> Option<char> {
        char::from_u32(self.code & 0xFFFF)
    }
}

#[cfg(test)]
#[path = "tests/character.rs"]
mod tests;
