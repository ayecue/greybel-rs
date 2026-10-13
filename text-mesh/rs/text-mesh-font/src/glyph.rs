use crate::atlas::GlyphRect;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlyphMetrics {
    pub width: f32,
    pub height: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Glyph {
    pub character: char,
    pub metrics: GlyphMetrics,
    pub rect: GlyphRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlyphAdjustment {
    pub x_placement: f32,
    pub y_placement: f32,
    pub x_advance: f32,
    pub y_advance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct KerningPair {
    pub first: GlyphAdjustment,
    pub second: GlyphAdjustment,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Kerning {
    pub first: char,
    pub second: char,
    pub pair: KerningPair,
}
