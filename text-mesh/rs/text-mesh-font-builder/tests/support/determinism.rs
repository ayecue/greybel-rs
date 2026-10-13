use text_mesh_font_builder::{BuiltFont, RustSource};

pub struct Output {
    pub source: String,
    pub pngs: Vec<Vec<u8>>,
}

impl Output {
    pub fn of(built: &BuiltFont) -> Self {
        Self {
            source: RustSource::new(&built.asset).to_string(),
            pngs: built
                .atlases
                .iter()
                .map(|atlas| atlas.encode_png().expect("the atlas encodes"))
                .collect(),
        }
    }
}
