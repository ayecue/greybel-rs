mod anti_aliased_distance;
mod envelope;
mod sampled_distance;

pub use anti_aliased_distance::AntiAliasedDistance;
pub use sampled_distance::SampledDistance;

/// Converts signed distance to TMP atlas alpha using the gradient spread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistanceScale {
    spread: f64,
}

impl DistanceScale {
    #[must_use]
    pub fn new(spread: i64) -> Self {
        Self {
            spread: spread.max(1) as f64,
        }
    }

    #[must_use]
    pub fn alpha(self, distance: f64) -> u8 {
        ((0.5 + distance / (2.0 * self.spread)) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8
    }

    #[must_use]
    pub const fn far_alpha(self, inside: bool) -> u8 {
        if inside { 255 } else { 0 }
    }
}
