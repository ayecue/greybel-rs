/// FreeType 16.16 fixed-point arithmetic with `FT_MulFix` and `FT_DivFix` rounding to 1/64-pixel metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedPoint(i64);

impl FixedPoint {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub fn ratio(numerator: i64, denominator: i64) -> Self {
        let negative = (numerator < 0) != (denominator < 0);
        let (numerator, denominator) = (numerator.abs(), denominator.abs().max(1));
        let quotient = ((numerator << 16) + (denominator >> 1)) / denominator;

        Self(if negative { -quotient } else { quotient })
    }

    #[must_use]
    pub const fn from_f2dot14(value: i16) -> Self {
        Self((value as i64) << 2)
    }

    #[must_use]
    pub const fn raw(self) -> i64 {
        self.0
    }

    #[must_use]
    pub fn hypot(self, other: Self) -> Self {
        Self(((self.0 as f64).hypot(other.0 as f64)).round() as i64)
    }

    #[must_use]
    pub const fn scale(self, value: i64) -> i64 {
        let negative = (value < 0) != (self.0 < 0);
        let product = (value.abs() * self.0.abs() + 0x8000) >> 16;

        if negative { -product } else { product }
    }
}

#[cfg(test)]
#[path = "tests/fixed_point.rs"]
mod tests;
