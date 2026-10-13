/// Provides Unity multisample pixel offsets with vertical flipping for render textures.
#[derive(Debug, Clone, Copy)]
pub struct SamplePattern;

impl SamplePattern {
    const EIGHT: [[f32; 2]; 8] = [
        [0.0625, 0.1875],
        [-0.0625, -0.1875],
        [0.3125, -0.0625],
        [-0.1875, 0.3125],
        [-0.3125, -0.3125],
        [-0.4375, 0.0625],
        [0.1875, -0.4375],
        [0.4375, 0.4375],
    ];
    const FOUR: [[f32; 2]; 4] = [
        [-0.125, 0.375],
        [0.375, 0.125],
        [-0.375, -0.125],
        [0.125, -0.375],
    ];
    const ONE: [[f32; 2]; 1] = [[0.0, 0.0]];
    const TWO: [[f32; 2]; 2] = [[0.25, -0.25], [-0.25, 0.25]];

    /// Offsets for `samples` samples per pixel (1, 2, 4 or 8; other
    /// counts take the next smaller pattern).
    #[must_use]
    pub const fn offsets(samples: u8) -> &'static [[f32; 2]] {
        match samples {
            0 | 1 => &Self::ONE,
            2 | 3 => &Self::TWO,
            4..=7 => &Self::FOUR,
            _ => &Self::EIGHT,
        }
    }
}
