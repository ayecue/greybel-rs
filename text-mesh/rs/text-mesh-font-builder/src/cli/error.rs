use std::path::PathBuf;

use text_mesh_font_builder::FontBuildError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path}: {message}")]
    Config { path: PathBuf, message: String },
    #[error("{path}: config files are .toml or .json")]
    ConfigFormat { path: PathBuf },
    #[error("no font file: pass --font or set `font` in the config")]
    MissingFont,
    #[error("`{0}` is not a code point or a range of code points (hex, like 20-7E)")]
    Range(String),
    #[error(
        "`{0}` is not a render mode (raster, raster-hinted, smooth, smooth-hinted, sdf, sdf8, sdf16, sdf32, sdfaa, sdfaa-hinted, or Unity's GlyphRenderMode number)"
    )]
    RenderMode(String),
    #[error("nothing to write: pass --rust and/or --atlases")]
    NoOutput,
    #[error(transparent)]
    Build(#[from] FontBuildError),
}
