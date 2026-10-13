use text_mesh_core::Alignment;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VerticalAlignment {
    #[default]
    Top,
    Middle,
    Bottom,
    Baseline,
    Geometry,
    Capline,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margin {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextSettings {
    pub font_size: f32,
    pub alignment: Alignment,
    pub vertical_alignment: VerticalAlignment,
    pub character_spacing: f32,
    pub word_spacing: f32,
    pub line_spacing: f32,
    pub line_spacing_max: f32,
    pub paragraph_spacing: f32,
    pub word_wrapping: bool,
    pub word_wrapping_ratios: f32,
    pub kerning: bool,
    pub margin: Margin,
}
