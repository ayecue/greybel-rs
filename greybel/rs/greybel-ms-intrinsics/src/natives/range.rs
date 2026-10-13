use greybel_interpreter::{Sequence, Value, native};

use crate::error::IntrinsicError;

const MAX_LIST_SIZE: i64 = 0xFF_FFFF;

/// Inclusive numeric range using additive steps, matching C# `range` rather than multiplicative steps.
#[derive(Debug, Clone)]
pub struct NumberRange {
    next: f64,
    to: f64,
    step: f64,
}

impl Iterator for NumberRange {
    type Item = f64;

    fn next(&mut self) -> Option<f64> {
        let value = self.next;
        let within = if self.step > 0.0 {
            value <= self.to
        } else {
            value >= self.to
        };

        if !within {
            return None;
        }

        self.next += self.step;

        Some(value)
    }
}

#[native]
pub fn range(
    #[default(0)] from: f64,
    #[default(0)] to: f64,
    step: &Value,
) -> Result<Sequence<NumberRange>, IntrinsicError> {
    let step = match step {
        Value::Number(step) => step.get(),
        _ if to >= from => 1.0,
        _ => -1.0,
    };

    if step == 0.0 {
        return Err(IntrinsicError::RangeStepZero);
    }

    #[allow(clippy::cast_possible_truncation)]
    let count = i64::from(((to - from) / step) as i32) + 1;

    if count > MAX_LIST_SIZE {
        return Err(IntrinsicError::ListTooLarge);
    }

    if count < 0 {
        return Err(IntrinsicError::Range);
    }

    let numbers = NumberRange {
        next: from,
        to,
        step,
    };
    let capacity = usize::try_from(count).unwrap_or_default();

    Ok(Sequence::with_capacity(numbers, capacity))
}
