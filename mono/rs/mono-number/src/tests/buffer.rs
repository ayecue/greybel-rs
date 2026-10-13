#[path = "support/buffer.rs"]
mod support;

use support::{reference, values};

use super::*;

#[test]
fn should_guard_digits_that_match_all_printed_digits() {
    let cases = values().flat_map(|value| [7, 9, 15, 17].map(|precision| (value, precision)));
    let outcomes = cases.map(|(value, precision)| {
        let fast = NumberBuffer::from_f64(value, precision);
        let exact = reference(value, precision);

        (value, precision, fast, exact)
    });

    for (value, precision, fast, exact) in outcomes {
        assert_eq!(
            (fast.scale, &fast.digits[..precision as usize]),
            (exact.scale, &exact.digits[..precision as usize]),
            "{value:e} with {precision} digits"
        );
    }
}
