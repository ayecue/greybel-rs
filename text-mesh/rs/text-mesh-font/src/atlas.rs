#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GlyphRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub atlas: u8,
}

/// TMP glyph render modes with their rasterization behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RenderMode {
    Raster,
    HintedRaster,
    Smooth,
    HintedSmooth,
    Sdf,
    Sdf8,
    #[default]
    Sdf16,
    Sdf32,
    Sdfaa,
    HintedSdfaa,
}

impl RenderMode {
    const UNITY: [(Self, u32); 10] = [
        (Self::Raster, 4101),
        (Self::HintedRaster, 4105),
        (Self::Smooth, 4117),
        (Self::HintedSmooth, 4121),
        (Self::Sdf, 4134),
        (Self::Sdf8, 8230),
        (Self::Sdf16, 16422),
        (Self::Sdf32, 32806),
        (Self::Sdfaa, 4165),
        (Self::HintedSdfaa, 4169),
    ];

    #[must_use]
    pub fn from_unity(value: u32) -> Option<Self> {
        Self::UNITY
            .iter()
            .find(|(_, unity)| *unity == value)
            .map(|(mode, _)| *mode)
    }

    #[must_use]
    pub fn unity(self) -> u32 {
        Self::UNITY
            .iter()
            .find(|(mode, _)| *mode == self)
            .map_or(0, |(_, unity)| *unity)
    }

    #[must_use]
    pub const fn is_distance_field(self) -> bool {
        matches!(
            self,
            Self::Sdf | Self::Sdf8 | Self::Sdf16 | Self::Sdf32 | Self::Sdfaa | Self::HintedSdfaa
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FontAtlas {
    pub width: u16,
    pub height: u16,
    pub padding: u16,
    pub render_mode: RenderMode,
    pub count: u8,
}
