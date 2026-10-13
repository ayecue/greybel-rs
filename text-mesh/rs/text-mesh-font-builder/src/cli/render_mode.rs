use text_mesh_font::RenderMode;

use super::error::CliError;

/// Render mode, by its TMP kebab-case name or Unity's `GlyphRenderMode` number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderModeName;

impl RenderModeName {
    const NAMES: [(&'static str, RenderMode); 10] = [
        ("raster", RenderMode::Raster),
        ("raster-hinted", RenderMode::HintedRaster),
        ("smooth", RenderMode::Smooth),
        ("smooth-hinted", RenderMode::HintedSmooth),
        ("sdf", RenderMode::Sdf),
        ("sdf8", RenderMode::Sdf8),
        ("sdf16", RenderMode::Sdf16),
        ("sdf32", RenderMode::Sdf32),
        ("sdfaa", RenderMode::Sdfaa),
        ("sdfaa-hinted", RenderMode::HintedSdfaa),
    ];

    /// # Errors
    /// Returns an error if the text does not identify a render mode.
    pub fn parse(text: &str) -> Result<RenderMode, CliError> {
        let name = text.trim().to_ascii_lowercase().replace('_', "-");

        if let Some((_, mode)) = Self::NAMES
            .iter()
            .find(|(entry_name, _)| *entry_name == name)
        {
            return Ok(*mode);
        }

        name.parse::<u32>()
            .ok()
            .and_then(RenderMode::from_unity)
            .ok_or_else(|| CliError::RenderMode(text.to_owned()))
    }

    #[must_use]
    pub fn of(mode: RenderMode) -> &'static str {
        Self::NAMES
            .iter()
            .find(|(_, candidate)| *candidate == mode)
            .map_or("sdfaa", |(name, _)| name)
    }
}
