/// Known differences from Unity per font and field, with the cause; a listed difference that disappears fails the test.
pub struct KnownDifference {
    pub font: &'static str,
    pub field: &'static str,
    pub mean: f64,
    pub max: u8,
    pub reason: &'static str,
}

pub const MONOCHROME: &str = "our monochrome rasterizer (pixel centers, non-zero winding, curves flattened to 1/50 px) \
    sets a few upsampled pixels differently from FreeType's (about 0.03% of them, at curve flattening and tie \
    positions); with FreeType's own bitmaps the measurement matches to within one level";

pub const ANTI_ALIASED: &str = "Unity takes the covered pixel nearest by center and adds its edge offset, as we do, but on a \
    few pixels (about 0.3%, at corners and faint edge pixels) it settles on the next-nearest one, so single pixels \
    differ by up to 21 levels; everywhere else the fields match to within a level";

pub const KNOWN_DIFFERENCES: &[KnownDifference] = &[
    KnownDifference {
        font: "FiraMono-Regular SDF",
        field: "atlas pixels",
        mean: 0.04,
        max: 20,
        reason: ANTI_ALIASED,
    },
    KnownDifference {
        font: "LiberationSans SDF",
        field: "atlas pixels",
        mean: 0.01,
        max: 1,
        reason: MONOCHROME,
    },
    KnownDifference {
        font: "Sarabun-Regular SDF",
        field: "atlas pixels",
        mean: 0.03,
        max: 3,
        reason: MONOCHROME,
    },
];

impl KnownDifference {
    pub fn of(font: &str, field: &str) -> Option<&'static Self> {
        KNOWN_DIFFERENCES
            .iter()
            .find(|known| known.font == font && known.field == field)
    }
}
