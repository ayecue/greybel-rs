use std::sync::LazyLock;

use crate::TERMINAL_TEXT;

/// Font atlas with one alpha byte per pixel, ordered by row from top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasFile {
    pub font: &'static str,
    pub index: u8,
    pub width: u16,
    pub height: u16,
    pub alpha: &'static [u8],
}

impl AtlasFile {
    /// Returns font atlases in font declaration order.
    #[must_use]
    pub fn all() -> &'static [Self] {
        static ALL: LazyLock<Vec<AtlasFile>> = LazyLock::new(|| {
            crate::bundle::SETUP
                .atlas_files()
                .into_iter()
                .map(|(font_name, index, file)| {
                    let font = TERMINAL_TEXT
                        .font_set
                        .fonts
                        .iter()
                        .find(|font| font.name == font_name)
                        .expect("every atlas belongs to a font of the setup");
                    let atlas = font.atlas.expect("a font with atlases has their size");

                    AtlasFile {
                        font: &font.name,
                        index,
                        width: atlas.width,
                        height: atlas.height,
                        alpha: crate::bundle::pixels("atlases", &file, "alpha"),
                    }
                })
                .collect()
        });

        &ALL
    }

    #[must_use]
    pub fn find(font: &str, index: u8) -> Option<&'static Self> {
        Self::all()
            .iter()
            .find(|atlas| atlas.font == font && atlas.index == index)
    }
}
