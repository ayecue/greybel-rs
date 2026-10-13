use thiserror::Error;

#[derive(Debug, Error)]
pub enum FontBuildError {
    #[error("the font cannot be read: {0}")]
    Font(ttf_parser::FaceParsingError),
    #[error("the atlas size {width}x{height} is unusable")]
    AtlasSize { width: u16, height: u16 },
    #[error("glyph {glyph} ({width}x{height} px with padding) does not fit in an empty atlas")]
    GlyphTooLarge { glyph: u16, width: u32, height: u32 },
    #[error("the atlas image cannot be encoded: {0}")]
    Png(#[from] png::EncodingError),
}
