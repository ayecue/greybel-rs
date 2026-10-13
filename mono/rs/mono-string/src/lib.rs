mod case;
pub mod collation;
pub mod utf16;

use std::cmp::Ordering;

pub use case::{to_lower, to_upper};

#[must_use]
pub fn compare_ordinal(a: &str, b: &str) -> Ordering {
    let Some(mismatch) = a.bytes().zip(b.bytes()).position(|(x, y)| x != y) else {
        return a.len().cmp(&b.len());
    };
    let mut start = mismatch;

    while !a.is_char_boundary(start) {
        start -= 1;
    }

    let units = |text: &str| {
        let mut units = [0u16; 2];
        let c = text[start..].chars().next().expect("a character");
        let len = c.encode_utf16(&mut units).len();

        (units, len)
    };
    let (x, x_len) = units(a);
    let (y, y_len) = units(b);

    x[..x_len].cmp(&y[..y_len])
}

#[must_use]
pub fn hash_code(text: &str) -> i32 {
    const MULTIPLIER: i32 = 1_566_083_941;
    let step = |hash: i32, unit: u16| ((hash << 5).wrapping_add(hash)) ^ i32::from(unit);
    let mut units = text.encode_utf16().take_while(|&unit| unit != 0);
    let (mut first, mut second) = (5381_i32, 5381_i32);

    while let Some(unit) = units.next() {
        first = step(first, unit);

        let Some(unit) = units.next() else { break };

        second = step(second, unit);
    }

    first.wrapping_add(second.wrapping_mul(MULTIPLIER))
}
