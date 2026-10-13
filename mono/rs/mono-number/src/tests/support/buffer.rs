use super::super::{DecimalDigitContainer, NumberBuffer, PRINTED_DECIMALS};

pub(super) fn reference(value: f64, precision: i32) -> NumberBuffer {
    let mut number = NumberBuffer::new();

    number.precision = precision;
    number.sign = value.is_sign_negative();

    let precision = precision as usize;
    let printed = DecimalDigitContainer::new(value, PRINTED_DECIMALS);

    number.scale = printed.exponent + 1;
    number.digits[..precision].copy_from_slice(&printed.digits[..precision]);
    if printed.digits[precision] >= b'5' {
        let mut index = precision - 1;

        while number.digits[index] == b'9' && index > 0 {
            number.digits[index] = b'0';
            index -= 1;
        }

        if index == 0 && number.digits[index] == b'9' {
            number.digits[index] = b'1';
            number.scale += 1;
        } else {
            number.digits[index] += 1;
        }
    }

    number
}

pub(super) fn values() -> impl Iterator<Item = f64> {
    let mut state = 0x5eed_u64;
    let mut next = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);

        let mut z = state;

        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);

        z ^ (z >> 31)
    };
    let ties = (0..2000_u64).flat_map(|k| {
        let integer = 1_000_000_000_000_000_u64 + k * 10 + 5;

        [
            integer as f64,
            (k as f64 + 0.5) / 1e6,
            (k * 2 + 1) as f64 / 1024.0,
        ]
    });
    let random = (0..200_000).map(move |_| f64::from_bits(next()));

    ties.chain(random)
        .filter(|value| value.is_finite() && *value != 0.0)
}
