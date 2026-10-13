use text_mesh_font::FontSet;

pub fn names(fonts: &FontSet) -> Vec<&str> {
    fonts.fonts.iter().map(|font| font.name.as_ref()).collect()
}
