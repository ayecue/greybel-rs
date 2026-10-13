use crate::outline::ControlBox;

/// Bitmap FreeType allocates for an outline, in full pixels with y up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelBox {
    pub left: i64,
    pub top: i64,
    pub width: i64,
    pub height: i64,
}

impl PixelBox {
    /// Applies monochrome pixel rounding and expands a collapsed box by one pixel toward the outline.
    #[must_use]
    pub fn monochrome(bounds: Option<ControlBox>) -> Self {
        let bounds = bounds.unwrap_or(ControlBox {
            x_min: 0,
            y_min: 0,
            x_max: 0,
            y_max: 0,
        });
        let (left, right) = Self::round_monochrome(bounds.x_min, bounds.x_max);
        let (bottom, top) = Self::round_monochrome(bounds.y_min, bounds.y_max);

        Self {
            left,
            top,
            width: right - left,
            height: top - bottom,
        }
    }

    /// Anti-aliasing rule: the edges move out to full pixels; an empty
    /// outline has an empty box.
    #[must_use]
    pub fn smooth(bounds: Option<ControlBox>) -> Self {
        let Some(bounds) = bounds else {
            return Self {
                left: 0,
                top: 0,
                width: 0,
                height: 0,
            };
        };
        let left = bounds.x_min.div_euclid(64);
        let right = (bounds.x_max + 63).div_euclid(64);
        let bottom = bounds.y_min.div_euclid(64);
        let top = (bounds.y_max + 63).div_euclid(64);

        Self {
            left,
            top,
            width: right - left,
            height: top - bottom,
        }
    }

    fn round_monochrome(min: i64, max: i64) -> (i64, i64) {
        let (min_pixel, min_part) = (min.div_euclid(64), min.rem_euclid(64));
        let (max_pixel, max_part) = (max.div_euclid(64), max.rem_euclid(64));
        let low = min_pixel + ((min_part + 31) >> 6);
        let high = max_pixel + ((max_part + 32) >> 6);

        if low != high {
            return (low, high);
        }

        if ((min_part + 31) & 63) - 31 + ((max_part + 32) & 63) - 32 < 0 {
            (low - 1, high)
        } else {
            (low, high + 1)
        }
    }
}

#[cfg(test)]
#[path = "tests/pixel_box.rs"]
mod tests;
