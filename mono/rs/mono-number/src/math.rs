const ROUND_LIMIT: f64 = 1e16;
const ROUND_POWERS: [f64; 16] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15,
];

#[must_use]
pub fn round_to_even(value: f64) -> f64 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    if value == value as i64 as f64 {
        return value;
    }

    let mut floor = (value + 0.5).floor();

    if value == floor - 0.5 && floor % 2.0 != 0.0 {
        floor -= 1.0;
    }

    floor.copysign(value)
}

#[must_use]
pub fn round(value: f64, digits: i32) -> Option<f64> {
    let power = *ROUND_POWERS.get(usize::try_from(digits).ok()?)?;

    if value.abs() >= ROUND_LIMIT {
        return Some(value);
    }

    Some(round_to_even(value * power) / power)
}

#[must_use]
pub fn hash_code(value: f64) -> i32 {
    const EXPONENT: u64 = 0x7FF0_0000_0000_0000;
    let mut bits = value.to_bits();

    if (bits.wrapping_sub(1) & !(1 << 63)) >= EXPONENT {
        bits &= EXPONENT;
    }

    #[allow(clippy::cast_possible_truncation)]
    let (low, high) = (bits as i32, (bits >> 32) as i32);

    low ^ high
}
