use std::io::Write;

pub(crate) const MAX_DIGITS: usize = 50;
pub(crate) const SCALE_NAN: i32 = i32::MIN;
pub(crate) const SCALE_INFINITY: i32 = i32::MAX;

const PRINTED_DECIMALS: usize = 40;
const PRINT_BUFFER: usize = 64;

#[derive(Debug, Clone)]
pub(crate) struct NumberBuffer {
    pub(crate) precision: i32,
    pub(crate) scale: i32,
    pub(crate) sign: bool,
    digits: [u8; MAX_DIGITS + 1],
}

impl NumberBuffer {
    pub(crate) fn new() -> Self {
        Self {
            precision: 0,
            scale: 0,
            sign: false,
            digits: [0; MAX_DIGITS + 1],
        }
    }

    pub(crate) fn digit(&self, index: usize) -> u8 {
        self.digits.get(index).copied().unwrap_or(0)
    }

    pub(crate) fn set_digit(&mut self, index: usize, digit: u8) {
        self.digits[index] = digit;
    }

    pub(crate) fn len(&self) -> usize {
        self.digits
            .iter()
            .position(|&digit| digit == 0)
            .unwrap_or(MAX_DIGITS)
    }

    pub(crate) fn is_nan(&self) -> bool {
        self.scale == SCALE_NAN
    }

    pub(crate) fn is_infinity(&self) -> bool {
        self.scale == SCALE_INFINITY
    }

    pub(crate) fn from_f64(value: f64, precision: i32) -> Self {
        debug_assert!(precision > 1 && precision < 40);

        let mut number = Self::new();

        number.precision = precision;
        number.sign = value.is_sign_negative();
        if !value.is_finite() {
            number.scale = if value.is_nan() {
                SCALE_NAN
            } else {
                SCALE_INFINITY
            };

            return number;
        }

        let precision = usize::try_from(precision).expect("a positive precision");

        if value.to_bits() == 0 {
            number.digits[..precision].fill(b'0');

            return number;
        }

        let shortest = (value != 0.0).then(|| Shortest::new(value));

        if let Some((shortest, round_up)) =
            shortest.and_then(|shortest| Some((shortest, shortest.rounds_up(value, precision)?)))
        {
            number.scale = shortest.exponent + 1;

            let digit_count = shortest.len.min(precision);

            number.digits[..digit_count].copy_from_slice(&shortest.digits[..digit_count]);
            number.digits[digit_count..precision].fill(b'0');
            number.digits[precision] = 0;
            if round_up {
                number.round_up(precision);
            }

            return number;
        }

        let guard_digits = DecimalDigitContainer::new(value, precision + 1);
        let guard = (guard_digits.digits[precision] - b'0') * 10
            + (guard_digits.digits[precision + 1] - b'0');
        let (final_digits, round_up) = if matches!(guard, 49 | 50) {
            let exact = DecimalDigitContainer::new(value, PRINTED_DECIMALS);
            let round_up = exact.digits[precision] >= b'5';

            (exact, round_up)
        } else {
            (guard_digits, guard > 50)
        };

        number.scale = final_digits.exponent + 1;
        number.digits[..precision].copy_from_slice(&final_digits.digits[..precision]);
        number.digits[precision] = 0;
        if round_up {
            number.round_up(precision);
        }

        number
    }

    fn round_up(&mut self, precision: usize) {
        let mut index = precision - 1;

        while self.digits[index] == b'9' && index > 0 {
            self.digits[index] = b'0';
            index -= 1;
        }

        if index == 0 && self.digits[index] == b'9' {
            self.digits[index] = b'1';
            self.scale += 1;
        } else {
            self.digits[index] += 1;
        }
    }

    pub(crate) fn to_f64(&self) -> f64 {
        let total = self.len();
        let mut start = 0;

        while self.digit(start) == b'0' {
            start += 1;
        }

        let mut remaining_digits = total - start;

        if remaining_digits == 0 {
            return 0.0;
        }

        let count = remaining_digits.min(9);

        remaining_digits -= count;

        let mut value = u64::from(self.digits_to_int(start, count));

        if remaining_digits > 0 {
            let count = remaining_digits.min(9);

            remaining_digits -= count;

            let mult = (POWERS_OF_10[count - 1] >> (64 - EXPONENTS_OF_10[count - 1])) as u32;

            value = u64::from(value as u32) * u64::from(mult)
                + u64::from(self.digits_to_int(start + 9, count));
        }

        let digit_span = i32::try_from(total - remaining_digits).expect("at most 50 digits");
        let scale = self.scale - digit_span;
        let magnitude = scale.unsigned_abs() as usize;

        if magnitude >= 22 * 16 {
            let mut bits = if scale > 0 {
                f64::INFINITY.to_bits()
            } else {
                0
            };

            if self.sign {
                bits |= SIGN_BIT;
            }

            return f64::from_bits(bits);
        }

        let mut exponent: i32 = 64;

        for shift in [32, 16, 8, 4, 2, 1] {
            let top = !(u64::MAX >> shift);

            if value & top == 0 {
                value <<= shift;
                exponent -= shift;
            }
        }

        let index = magnitude & 15;

        if index != 0 {
            let power_exponent = i32::from(EXPONENTS_OF_10[index - 1]);

            exponent += if scale < 0 {
                -power_exponent + 1
            } else {
                power_exponent
            };

            let power = POWERS_OF_10[index + if scale < 0 { 15 } else { 0 } - 1];

            value = mul64_lossy(value, power, &mut exponent);
        }

        let index = magnitude >> 4;

        if index != 0 {
            let power_exponent = i32::from(EXPONENTS_OF_10_BY_16[index - 1]);

            exponent += if scale < 0 {
                -power_exponent + 1
            } else {
                power_exponent
            };

            let power = POWERS_OF_10_BY_16[index + if scale < 0 { 21 } else { 0 } - 1];

            value = mul64_lossy(value, power, &mut exponent);
        }

        if value & (1 << 10) != 0 {
            let round_bits = value
                .wrapping_add((1 << 10) - 1)
                .wrapping_add((value >> 11) & 1);

            if round_bits < value {
                value = (round_bits >> 1) | SIGN_BIT;
                exponent += 1;
            } else {
                value = round_bits;
            }
        }

        exponent += 0x3FE;

        let mut bits = if exponent <= 0 {
            if exponent == -52 && value >= 0x8000_0000_0000_0058 {
                1
            } else if exponent <= -52 {
                0
            } else {
                value >> (-exponent + 11 + 1)
            }
        } else if exponent >= 0x7FF {
            f64::INFINITY.to_bits()
        } else {
            (u64::try_from(exponent).expect("positive") << 52) + ((value >> 11) & MANTISSA)
        };

        if self.sign {
            bits |= SIGN_BIT;
        }

        f64::from_bits(bits)
    }

    fn digits_to_int(&self, start: usize, count: usize) -> u32 {
        self.digits[start..start + count]
            .iter()
            .fold(0u32, |value, &digit| {
                value
                    .wrapping_mul(10)
                    .wrapping_add(u32::from(digit.wrapping_sub(b'0')))
            })
    }
}

const SIGN_BIT: u64 = 1 << 63;
const MANTISSA: u64 = (1 << 52) - 1;

fn mul64_lossy(a: u64, b: u64, exponent: &mut i32) -> u64 {
    let (a_high, a_low) = (a >> 32, a & 0xFFFF_FFFF);
    let (b_high, b_low) = (b >> 32, b & 0xFFFF_FFFF);
    let mut value = (a_high * b_high)
        .wrapping_add((a_high * b_low) >> 32)
        .wrapping_add((a_low * b_high) >> 32);

    if value & SIGN_BIT == 0 {
        value <<= 1;
        *exponent -= 1;
    }

    value
}

const POWERS_OF_10: [u64; 30] = [
    0xa000_0000_0000_0000,
    0xc800_0000_0000_0000,
    0xfa00_0000_0000_0000,
    0x9c40_0000_0000_0000,
    0xc350_0000_0000_0000,
    0xf424_0000_0000_0000,
    0x9896_8000_0000_0000,
    0xbebc_2000_0000_0000,
    0xee6b_2800_0000_0000,
    0x9502_f900_0000_0000,
    0xba43_b740_0000_0000,
    0xe8d4_a510_0000_0000,
    0x9184_e72a_0000_0000,
    0xb5e6_20f4_8000_0000,
    0xe35f_a931_a000_0000,
    0xcccc_cccc_cccc_cccd,
    0xa3d7_0a3d_70a3_d70b,
    0x8312_6e97_8d4f_df3c,
    0xd1b7_1758_e219_652e,
    0xa7c5_ac47_1b47_8425,
    0x8637_bd05_af6c_69b7,
    0xd6bf_94d5_e57a_42be,
    0xabcc_7711_8461_ceff,
    0x8970_5f41_36b4_a599,
    0xdbe6_fece_bded_d5c2,
    0xafeb_ff0b_cb24_ab02,
    0x8cbc_cc09_6f50_88cf,
    0xe12e_1342_4bb4_0e18,
    0xb424_dc35_095c_d813,
    0x901d_7cf7_3ab0_acdc,
];

const EXPONENTS_OF_10: [i8; 15] = [4, 7, 10, 14, 17, 20, 24, 27, 30, 34, 37, 40, 44, 47, 50];

const POWERS_OF_10_BY_16: [u64; 42] = [
    0x8e1b_c9bf_0400_0000,
    0x9dc5_ada8_2b70_b59e,
    0xaf29_8d05_0e43_95d6,
    0xc278_1f49_ffcf_a6d4,
    0xd7e7_7a8f_87da_f7fa,
    0xefb3_ab16_c59b_14a0,
    0x850f_adc0_9923_329c,
    0x93ba_47c9_80e9_8cde,
    0xa402_b9c5_a8d3_a6e6,
    0xb616_a12b_7fe6_17a8,
    0xca28_a291_859b_bf90,
    0xe070_f78d_3927_5566,
    0xf92e_0c35_3782_6140,
    0x8a52_96ff_e33c_c92c,
    0x9991_a6f3_d6bf_1762,
    0xaa7e_ebfb_9df9_de8a,
    0xbd49_d14a_a79d_bc7e,
    0xd226_fc19_5c6a_2f88,
    0xe950_df20_247c_83f8,
    0x8184_2f29_f2cc_e373,
    0x8fca_c257_558e_e4e2,
    0xe695_94be_c44d_e160,
    0xcfb1_1ead_4539_94c3,
    0xbb12_7c53_b17e_c165,
    0xa87f_ea27_a539_e9b3,
    0x97c5_60ba_6b09_19b5,
    0x88b4_02f7_fd75_53ab,
    0xf643_35bc_f065_d3a0,
    0xddd0_467c_64bc_e4c4,
    0xc7ca_ba6e_7c53_82ed,
    0xb3f4_e093_db73_a0b7,
    0xa217_27db_38cb_0053,
    0x91ff_8377_5423_cc29,
    0x8380_dea9_3da4_bc82,
    0xece5_3cec_4a31_4f00,
    0xd560_5fcd_cf32_e217,
    0xc031_4325_637a_1978,
    0xad1c_8eab_5ee4_3ba2,
    0x9bec_ce62_836a_c5b0,
    0x8c71_dcd9_ba0b_495c,
    0xfd00_b897_4782_3938,
    0xe3e2_7a44_4d8d_991a,
];

const EXPONENTS_OF_10_BY_16: [i16; 21] = [
    54, 107, 160, 213, 266, 319, 373, 426, 479, 532, 585, 638, 691, 745, 798, 851, 904, 957, 1010,
    1064, 1117,
];

#[derive(Clone, Copy)]
struct Shortest {
    digits: [u8; SHORTEST_DIGITS],
    len: usize,
    exponent: i32,
}

const SHORTEST_DIGITS: usize = 17;
const MARGIN_SLACK: f64 = 1.0 + 1e-6;
const EXACT_DECIMALS: usize = f64::DIGITS as usize;

impl Shortest {
    fn new(value: f64) -> Self {
        let mut buffer = ryu::Buffer::new();
        let text = buffer.format_finite(value.abs()).as_bytes();
        let (mantissa, exponent) = match text.iter().position(|&byte| byte == b'e') {
            Some(marker) => (
                &text[..marker],
                std::str::from_utf8(&text[marker + 1..])
                    .ok()
                    .and_then(|exponent| exponent.parse::<i32>().ok())
                    .expect("a decimal exponent"),
            ),
            None => (text, 0),
        };
        let integer_digits = mantissa
            .iter()
            .position(|&byte| byte == b'.')
            .unwrap_or(mantissa.len());
        let mut digits = [b'0'; SHORTEST_DIGITS];
        let mut len = 0;
        let mut first = None;

        for (index, &byte) in mantissa
            .iter()
            .filter(|byte| byte.is_ascii_digit())
            .enumerate()
        {
            if first.is_none() && byte == b'0' {
                continue;
            }

            first.get_or_insert(index);
            digits[len] = byte;
            len += 1;
        }

        while len > 0 && digits[len - 1] == b'0' {
            len -= 1;
        }

        let first = i32::try_from(first.expect("a non-zero value")).expect("few digits");
        let integer_digits = i32::try_from(integer_digits).expect("few digits");

        Self {
            digits,
            len,
            exponent: integer_digits - 1 - first + exponent,
        }
    }

    fn rounds_up(&self, value: f64, precision: usize) -> Option<bool> {
        if self.len <= precision {
            return (precision <= EXACT_DECIMALS && value.is_normal()).then_some(false);
        }

        let tail = &self.digits[precision..self.len];
        let tail_value = tail
            .iter()
            .fold(0u64, |total, &digit| total * 10 + u64::from(digit - b'0'));
        let tail_len = i32::try_from(tail.len()).expect("few digits");
        #[allow(clippy::cast_precision_loss)]
        let fraction = tail_value as f64 / 10f64.powi(tail_len);
        let magnitude = value.abs();
        let half_ulp = (magnitude.next_up() - magnitude) / 2.0;
        let unit = 10f64.powi(self.exponent - i32::try_from(precision).expect("few digits") + 1);
        let margin = half_ulp / unit * MARGIN_SLACK;

        ((fraction - 0.5).abs() > margin).then_some(fraction > 0.5)
    }
}

struct DecimalDigitContainer {
    digits: [u8; PRINTED_DECIMALS + 1],
    exponent: i32,
}

impl DecimalDigitContainer {
    fn new(value: f64, decimals: usize) -> Self {
        let mut text_buffer = [0u8; PRINT_BUFFER];
        let len = {
            let mut cursor = &mut text_buffer[..];

            write!(cursor, "{value:.decimals$e}").expect("fits the buffer");

            PRINT_BUFFER - cursor.len()
        };
        let text = &text_buffer[..len];
        let marker = text
            .iter()
            .rposition(|&byte| byte == b'e')
            .expect("an exponent");
        let exponent = std::str::from_utf8(&text[marker + 1..])
            .ok()
            .and_then(|exponent| exponent.parse().ok())
            .expect("a decimal exponent");
        let mut digits = [b'0'; PRINTED_DECIMALS + 1];

        for (slot, &byte) in digits
            .iter_mut()
            .zip(text[..marker].iter().filter(|byte| byte.is_ascii_digit()))
        {
            *slot = byte;
        }

        Self { digits, exponent }
    }
}

#[cfg(test)]
#[path = "tests/buffer.rs"]
mod tests;
