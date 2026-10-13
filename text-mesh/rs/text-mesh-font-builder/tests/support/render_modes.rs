use text_mesh_font::RenderMode;

pub const MODES: [RenderMode; 10] = [
    RenderMode::Raster,
    RenderMode::HintedRaster,
    RenderMode::Smooth,
    RenderMode::HintedSmooth,
    RenderMode::Sdf,
    RenderMode::Sdf8,
    RenderMode::Sdf16,
    RenderMode::Sdf32,
    RenderMode::Sdfaa,
    RenderMode::HintedSdfaa,
];
