#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FaceInfo {
    pub point_size: f32,
    pub scale: f32,
    pub line_height: f32,
    pub ascent_line: f32,
    pub cap_line: f32,
    pub mean_line: f32,
    pub baseline: f32,
    pub descent_line: f32,
    pub superscript_offset: f32,
    pub superscript_size: f32,
    pub subscript_offset: f32,
    pub subscript_size: f32,
    pub underline_offset: f32,
    pub underline_thickness: f32,
    pub strikethrough_offset: f32,
    pub strikethrough_thickness: f32,
    pub tab_width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FontStyleSettings {
    pub normal_style: f32,
    pub normal_spacing_offset: f32,
    pub bold_style: f32,
    pub bold_spacing: f32,
    pub italic_style: u8,
    pub tab_size: u8,
}
