use mono_number::NumberStyles;

use crate::error::IntrinsicError;

#[must_use]
pub fn parse(text: &str) -> f64 {
    mono_number::parse_f64(text, NumberStyles::ANY).unwrap_or(0.0)
}

#[must_use]
pub fn round(number: f64, places: i32) -> f64 {
    if places >= 0 {
        mono_number::round(number, places.min(MAX_DECIMALS)).expect("0 to 15 places")
    } else {
        let power = 10f64.powf(f64::from(-places));

        mono_number::round_to_even(number / power) * power
    }
}

const MAX_DECIMALS: i32 = 15;

pub fn sign(number: f64) -> Result<f64, IntrinsicError> {
    if number.is_nan() {
        return Err(IntrinsicError::NotANumber);
    }

    Ok(if number > 0.0 {
        1.0
    } else if number < 0.0 {
        -1.0
    } else {
        0.0
    })
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn sign_magnitude(number: f64) -> Result<(bool, u64), IntrinsicError> {
    Ok((sign(number)? < 0.0, number.abs() as u64))
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn from_sign_magnitude(negative: bool, magnitude: u64) -> f64 {
    let value = magnitude as f64;

    if negative { -value } else { value }
}

#[cfg(test)]
#[path = "tests/number.rs"]
mod tests;
