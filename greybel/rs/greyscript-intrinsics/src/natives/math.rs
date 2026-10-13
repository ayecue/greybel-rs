use greybel_interpreter::{Value, native};

use crate::error::IntrinsicError;

const NOT: &str = "~";
const AND: &str = "&";
const OR: &str = "|";
const XOR: &str = "^";
const SHIFT_LEFT: &str = "<<";
const SHIFT_RIGHT: &str = ">>";
const UNSIGNED_SHIFT_RIGHT: &str = ">>>";

/// Implements `bitwise` on 64-bit integers with C#-compatible shift counts.
#[native]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn bitwise(operator: &Value, num1: &Value, num2: f64) -> Option<f64> {
    let (Some(operator), Some(a)) = (operator.as_str(), num1.as_number()) else {
        return None;
    };
    let a = a as i64;
    let b = num2 as i64;
    let shift = b as i32 as u32;
    let result = match operator {
        NOT => !a,
        AND => a & b,
        OR => a | b,
        XOR => a ^ b,
        SHIFT_LEFT => a.wrapping_shl(shift),
        SHIFT_RIGHT => a.wrapping_shr(shift),
        UNSIGNED_SHIFT_RIGHT => (a as u64).wrapping_shr(shift) as i64,
        _ => return None,
    };

    Some(result as f64)
}

/// Applies 32-bit integer bit operators using Grey Hack semantics, not MiniScript's sign-and-magnitude semantics.
#[native]
pub fn bit_and(#[default(0)] i: i32, #[default(0)] j: i32) -> f64 {
    f64::from(i & j)
}

#[native]
pub fn bit_or(#[default(0)] i: i32, #[default(0)] j: i32) -> f64 {
    f64::from(i | j)
}

#[native]
pub fn bit_xor(#[default(0)] i: i32, #[default(0)] j: i32) -> f64 {
    f64::from(i ^ j)
}

/// Implements `sign`; Grey Hack's `Math.Sign` throws on NaN, crashing the script rather than raising a runtime error.
#[native]
pub fn sign(#[default(0)] x: f64) -> Result<f64, IntrinsicError> {
    greybel_ms_intrinsics::natives::math::sign(x).map_err(|_| IntrinsicError::GameCrash)
}
