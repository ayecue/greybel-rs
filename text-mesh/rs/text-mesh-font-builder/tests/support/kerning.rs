use text_mesh_font::GlyphAdjustment;

/// An adjustment rounded to 1/64 px, as FreeType's 26.6 fixed point has it.
pub fn to_64ths(adjustment: GlyphAdjustment) -> GlyphAdjustment {
    let round = |value: f32| (value * 64.0).round() / 64.0;

    GlyphAdjustment {
        x_placement: round(adjustment.x_placement),
        y_placement: round(adjustment.y_placement),
        x_advance: round(adjustment.x_advance),
        y_advance: round(adjustment.y_advance),
    }
}
