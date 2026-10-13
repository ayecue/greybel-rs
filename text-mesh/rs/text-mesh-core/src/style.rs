use crate::color::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const BLACK: Self = Self(900);
    pub const BOLD: Self = Self(700);
    pub const EXTRA_LIGHT: Self = Self(200);
    pub const HEAVY: Self = Self(800);
    pub const LIGHT: Self = Self(300);
    pub const MEDIUM: Self = Self(500);
    pub const REGULAR: Self = Self(400);
    pub const SEMI_BOLD: Self = Self(600);
    pub const THIN: Self = Self(100);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub color: Color,
    pub bold: bool,
    pub weight: FontWeight,
    pub italic: bool,
    pub underline: Option<Color>,
    pub strikethrough: Option<Color>,
    pub highlight: Option<Color>,
    pub size: f32,
    pub subscript: bool,
    pub superscript: bool,
    pub small_caps: bool,
    pub character_spacing: f32,
    pub mono_spacing: f32,
    pub vertical_offset: f32,
    pub rotation: f32,
    pub scale: f32,
    pub font: Option<u16>,
    pub material: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Pixels(f32),
    Percentage(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
    Left,
    Center,
    Right,
    Justified,
    Flush,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineHeight {
    Pixels(f32),
    Percentage {
        value: f32,
        size: f32,
        font: Option<u16>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutChange {
    Indent(Length),
    IndentEnd,
    LineIndent(Length),
    LineIndentEnd,
    Margin {
        left: Option<Length>,
        right: Option<Length>,
    },
    MarginEnd,
    LineHeight(LineHeight),
    LineHeightEnd,
    Width(Length),
    WidthEnd,
    Align(Alignment),
    AlignEnd,
    NoBreak(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellMetricContainer {
    pub advance: f32,
    pub line_height: f32,
}

impl CellMetricContainer {
    pub const MONOSPACE: Self = Self {
        advance: 0.6,
        line_height: 1.25,
    };
}

impl Default for CellMetricContainer {
    fn default() -> Self {
        Self::MONOSPACE
    }
}
