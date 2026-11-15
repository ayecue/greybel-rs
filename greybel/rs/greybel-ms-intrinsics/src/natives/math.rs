use std::f64::consts::PI;

use greybel_interpreter::native;

use crate::error::IntrinsicError;
use crate::number;

#[allow(clippy::approx_constant)]
const E: f64 = 2.718_282;
const E_TOLERANCE: f64 = 0.000_001;

#[native]
pub fn abs(#[default(0)] x: f64) -> f64 {
    x.abs()
}

#[native]
pub fn acos(#[default(0)] x: f64) -> f64 {
    x.acos()
}

#[native]
pub fn asin(#[default(0)] x: f64) -> f64 {
    x.asin()
}

#[native]
#[expect(
    clippy::float_cmp,
    reason = "The exact default argument selects MiniScript's one-argument arctangent"
)]
pub fn atan(#[default(0)] y: f64, #[default(1)] x: f64) -> f64 {
    if x == 1.0 { y.atan() } else { y.atan2(x) }
}

fn bits(
    i: f64,
    j: f64,
    sign: fn(bool, bool) -> bool,
    magnitude: fn(u64, u64) -> u64,
) -> Result<f64, IntrinsicError> {
    let (a_negative, a) = number::sign_magnitude(i)?;
    let (b_negative, b) = number::sign_magnitude(j)?;

    Ok(number::from_sign_magnitude(
        sign(a_negative, b_negative),
        magnitude(a, b),
    ))
}

#[native]
pub fn bit_and(#[default(0)] i: f64, #[default(0)] j: f64) -> Result<f64, IntrinsicError> {
    bits(i, j, |a, b| a & b, |a, b| a & b)
}

#[native]
pub fn bit_or(#[default(0)] i: f64, #[default(0)] j: f64) -> Result<f64, IntrinsicError> {
    bits(i, j, |a, b| a | b, |a, b| a | b)
}

#[native]
pub fn bit_xor(#[default(0)] i: f64, #[default(0)] j: f64) -> Result<f64, IntrinsicError> {
    bits(i, j, |a, b| a ^ b, |a, b| a ^ b)
}

#[native]
pub fn ceil(#[default(0)] x: f64) -> f64 {
    x.ceil()
}

#[native]
pub fn cos(#[default(0)] radians: f64) -> f64 {
    radians.cos()
}

#[native]
pub fn floor(#[default(0)] x: f64) -> f64 {
    x.floor()
}

#[native]
pub fn log(#[default(0)] x: f64, #[default(10)] base: f64) -> f64 {
    if (base - E).abs() < E_TOLERANCE {
        x.ln()
    } else {
        x.ln() / base.ln()
    }
}

#[native]
pub fn pi() -> f64 {
    PI
}

#[native]
pub fn round(
    #[default(0)] x: f64,
    #[default(0)]
    #[name = "decimalPlaces"]
    places: i32,
) -> f64 {
    number::round(x, places)
}

#[native]
pub fn sign(#[default(0)] x: f64) -> Result<f64, IntrinsicError> {
    number::sign(x)
}

#[native]
pub fn sin(#[default(0)] radians: f64) -> f64 {
    radians.sin()
}

#[native]
pub fn sqrt(#[default(0)] x: f64) -> f64 {
    x.sqrt()
}

#[native]
pub fn tan(#[default(0)] radians: f64) -> f64 {
    radians.tan()
}
