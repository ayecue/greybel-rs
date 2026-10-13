use text_mesh_core::{Alignment, Color, Style};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElementKind {
    Glyph { character: char, font: usize },
    Sprite { index: u32, color: Color },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Element {
    pub kind: ElementKind,
    pub style: Style,
    pub line: usize,
    pub visible: bool,
    pub origin: f32,
    pub advance: f32,
    pub baseline: f32,
    pub ascender: f32,
    pub descender: f32,
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub point_size: f32,
    pub scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub first: usize,
    pub last: usize,
    pub ascender: f32,
    pub descender: f32,
    pub baseline: f32,
    pub line_height: f32,
    pub width: f32,
    pub length: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    pub alignment: Alignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// Corners of an element's quad in TMP's vertex order (BL, TL, TR, BR), padded, sheared for italics and rotated or scaled.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Quad {
    pub bottom_left: Position,
    pub top_left: Position,
    pub top_right: Position,
    pub bottom_right: Position,
}

/// Additional TMP vertex-buffer data for an [`Element`]: font, base font, and quad and UV padding.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ElementGeometry {
    pub quad: Quad,
    pub font: usize,
    pub base_font: Option<usize>,
    pub padding: f32,
    pub style_padding: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LineGeometry {
    pub first_visible: usize,
    pub last_visible: usize,
}

/// Per-element and per-line [`Layout`] data consumed by TMP mesh generation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayoutGeometry {
    pub elements: Vec<ElementGeometry>,
    pub lines: Vec<LineGeometry>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layout {
    pub width: f32,
    pub height: f32,
    pub preferred_height: f32,
    pub elements: Vec<Element>,
    pub lines: Vec<Line>,
}
