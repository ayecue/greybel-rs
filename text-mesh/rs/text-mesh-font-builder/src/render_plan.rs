use text_mesh_font::RenderMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rasterization {
    Monochrome,
    Coverage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceField {
    None,
    Bitmap,
    Coverage,
}

/// Glyph rasterization size and distance-field algorithm for a render mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderPlan {
    pub rasterization: Rasterization,
    pub distance: DistanceField,
    pub upsampling: u16,
    pub hinted: bool,
}

impl RenderPlan {
    #[must_use]
    pub const fn of(mode: RenderMode) -> Self {
        let (rasterization, distance, upsampling, hinted) = match mode {
            RenderMode::Raster => (Rasterization::Monochrome, DistanceField::None, 1, false),
            RenderMode::HintedRaster => (Rasterization::Monochrome, DistanceField::None, 1, true),
            RenderMode::Smooth => (Rasterization::Coverage, DistanceField::None, 1, false),
            RenderMode::HintedSmooth => (Rasterization::Coverage, DistanceField::None, 1, true),
            RenderMode::Sdf => (Rasterization::Monochrome, DistanceField::Bitmap, 1, false),
            RenderMode::Sdf8 => (Rasterization::Monochrome, DistanceField::Bitmap, 8, false),
            RenderMode::Sdf16 => (Rasterization::Monochrome, DistanceField::Bitmap, 16, false),
            RenderMode::Sdf32 => (Rasterization::Monochrome, DistanceField::Bitmap, 32, false),
            RenderMode::Sdfaa => (Rasterization::Coverage, DistanceField::Coverage, 1, false),
            RenderMode::HintedSdfaa => (Rasterization::Coverage, DistanceField::Coverage, 1, true),
        };

        Self {
            rasterization,
            distance,
            upsampling,
            hinted,
        }
    }

    /// One-pixel distance-field margin on the atlas's right and top edges, added to every packed glyph.
    #[must_use]
    pub const fn packing_margin(self) -> u16 {
        match self.distance {
            DistanceField::None => 0,
            DistanceField::Bitmap | DistanceField::Coverage => 1,
        }
    }
}
