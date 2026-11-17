mod display;

use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

use greybel_core::BitwiseOp;

const EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[repr(transparent)]
pub struct Number(f64);

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }

    fn lt(&self, other: &Self) -> bool {
        self.0 < other.0
    }

    fn le(&self, other: &Self) -> bool {
        self.0 <= other.0
    }

    fn gt(&self, other: &Self) -> bool {
        self.0 > other.0
    }

    fn ge(&self, other: &Self) -> bool {
        self.0 >= other.0
    }
}

impl Number {
    pub const ONE: Self = Self(1.0);
    pub const ZERO: Self = Self(0.0);

    #[must_use]
    pub const fn new(number: f64) -> Self {
        Self(number)
    }

    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    #[must_use]
    pub fn to_int(self) -> i32 {
        #[allow(clippy::cast_possible_truncation)]
        let int = self.0 as i64 as i32;

        int
    }

    #[must_use]
    pub fn to_index(self) -> Option<usize> {
        if !self.0.is_finite() || self.0 < 0.0 {
            return None;
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(self.0 as usize)
    }

    #[must_use]
    pub fn repeat_count(self, len: usize) -> Option<usize> {
        if !self.0.is_finite() {
            return None;
        }

        if len == 0 || self.0 <= 0.0 {
            return Some(0);
        }

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        Some((len as f64 * self.0).floor() as usize)
    }

    #[must_use]
    pub fn is_truthy(self) -> bool {
        self.0 != 0.0
    }

    #[must_use]
    pub fn is_identical(self, other: Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }

    pub(crate) fn key_bits(self) -> u64 {
        if self.0 == 0.0 {
            0
        } else if self.0.is_nan() {
            f64::NAN.to_bits()
        } else {
            self.0.to_bits()
        }
    }

    #[must_use]
    pub fn pow(self, exponent: Self) -> Self {
        Self(self.0.powf(exponent.0))
    }

    #[must_use]
    pub fn clamp_truth(self) -> Self {
        let magnitude = self.0.abs();

        Self(if magnitude > 1.0 { 1.0 } else { magnitude })
    }

    #[must_use]
    pub fn fuzzy_and(self, other: Self) -> Self {
        (self * other).clamp_truth()
    }

    #[must_use]
    pub fn fuzzy_or(self, other: Self) -> Self {
        (self + other - self * other).clamp_truth()
    }

    #[must_use]
    pub fn bitwise(self, operator: BitwiseOp, other: Self) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        let (x, y) = (self.0 as i64, other.0 as i64);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let shift = y as u32;
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
        let result = match operator {
            BitwiseOp::And => x & y,
            BitwiseOp::Or => x | y,
            BitwiseOp::ShiftLeft => x.wrapping_shl(shift),
            BitwiseOp::ShiftRight => x.wrapping_shr(shift),
            BitwiseOp::UnsignedShiftRight => (x as u64).wrapping_shr(shift) as i64,
        };

        #[allow(clippy::cast_precision_loss)]
        Self(result as f64)
    }
}

impl From<f64> for Number {
    fn from(number: f64) -> Self {
        Self(number)
    }
}

impl From<bool> for Number {
    fn from(value: bool) -> Self {
        Self(f64::from(u8::from(value)))
    }
}

impl From<Number> for f64 {
    fn from(number: Number) -> Self {
        number.0
    }
}

impl Add for Number {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Sub for Number {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl Mul for Number {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }
}

impl Div for Number {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Self(self.0 / other.0)
    }
}

impl Rem for Number {
    type Output = Self;

    fn rem(self, other: Self) -> Self {
        let (x, y) = (self.0, other.0);

        if x.fract() == 0.0
            && y.fract() == 0.0
            && x.abs() < EXACT_INTEGER
            && y.abs() < EXACT_INTEGER
            && y != 0.0
        {
            #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
            let result = ((x as i64) % (y as i64)) as f64;

            return if result == 0.0 {
                Self(0.0_f64.copysign(x))
            } else {
                Self(result)
            };
        }

        Self(x % y)
    }
}

impl Neg for Number {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}
