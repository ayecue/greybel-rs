use text_mesh_core::Alignment;

use crate::layout::ElementKind;
use crate::text_layout::records::{Extents, LineInfo, TextLimits};
use crate::text_layout::source::SourceElement;
use crate::text_layout::tags::TagState;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cursor {
    pub tags: TagState,
    pub line_number: usize,
    pub line_offset: f32,
    pub max_line_ascender: f32,
    pub max_line_descender: f32,
    pub start_of_line_ascender: f32,
    pub max_text_ascender: f32,
    pub max_cap_height: f32,
    pub element_descender: f32,
    pub is_driven_line_spacing: bool,
    pub first_character_of_line: usize,
    pub first_visible_character_of_line: usize,
    pub last_visible_character_of_line: usize,
    pub line_visible_character_count: usize,
    pub current_font: usize,
    pub current_base_font: usize,
    pub italic_angle: Option<f32>,
}

impl Cursor {
    pub(crate) fn new(alignment: Alignment, primary: usize) -> Self {
        Self {
            tags: TagState::new(alignment),
            line_number: 0,
            line_offset: 0.0,
            max_line_ascender: TextLimits::LARGE_NEGATIVE,
            max_line_descender: TextLimits::LARGE_POSITIVE,
            start_of_line_ascender: 0.0,
            max_text_ascender: 0.0,
            max_cap_height: 0.0,
            element_descender: 0.0,
            is_driven_line_spacing: false,
            first_character_of_line: 0,
            first_visible_character_of_line: 0,
            last_visible_character_of_line: 0,
            line_visible_character_count: 0,
            current_font: primary,
            current_base_font: primary,
            italic_angle: None,
        }
    }

    pub(crate) const fn font_for(&self, element: &SourceElement) -> usize {
        match element.kind {
            ElementKind::Glyph { font, .. } => font,
            ElementKind::Sprite { .. } if self.current_base_font == element.font => {
                self.current_font
            }
            ElementKind::Sprite { .. } => element.font,
        }
    }

    pub(crate) fn start_line(&mut self) {
        self.max_line_ascender = TextLimits::LARGE_NEGATIVE;
        self.max_line_descender = TextLimits::LARGE_POSITIVE;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WrapState {
    pub index: isize,
    pub count: isize,
    pub cursor: Cursor,
    pub line: LineInfo,
    pub mesh: Extents,
}

impl WrapState {
    pub(crate) fn new(
        index: isize,
        count: isize,
        cursor: &Cursor,
        line: LineInfo,
        mesh: Extents,
    ) -> Self {
        Self {
            index,
            count,
            cursor: cursor.clone(),
            line,
            mesh,
        }
    }

    pub(crate) fn restore_into(&self, cursor: &mut Cursor) {
        let tags = &cursor.tags;
        let mut new_cursor = self.cursor.clone();

        new_cursor.tags.indent = tags.indent;
        new_cursor.tags.line_indent = tags.line_indent;
        new_cursor.tags.line_height = tags.line_height;
        new_cursor.tags.width = tags.width;
        *cursor = new_cursor;
    }
}
