#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SdfMaterial {
    pub gradient_scale: f32,
    pub face_dilate: f32,
    pub outline_width: f32,
    pub outline_softness: f32,
    pub weight_normal: f32,
    pub weight_bold: f32,
}

impl SdfMaterial {
    const CLAMP: f32 = 1.0;

    #[must_use]
    pub fn scale_ratio_a(&self) -> f32 {
        let weight = self.weight_normal.max(self.weight_bold) / 4.0;
        let t = 1.0_f32.max(weight + self.face_dilate + self.outline_width + self.outline_softness);

        (self.gradient_scale - Self::CLAMP) / (self.gradient_scale * t)
    }

    #[must_use]
    pub fn padding(&self) -> f32 {
        let ratio = self.scale_ratio_a();
        let face_dilate = self.face_dilate * ratio;
        let softness = self.outline_softness * ratio;
        let outline = self.outline_width * ratio;
        let uniform = (outline + softness + face_dilate).max(face_dilate);
        let padding = uniform.clamp(0.0, 1.0) * self.gradient_scale;

        padding + 1.25
    }

    #[must_use]
    pub const fn with_atlas_of(&self, target: &Self) -> Self {
        Self {
            gradient_scale: target.gradient_scale,
            weight_normal: target.weight_normal,
            weight_bold: target.weight_bold,
            ..*self
        }
    }
}
