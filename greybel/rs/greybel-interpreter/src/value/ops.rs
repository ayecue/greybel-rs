use std::borrow::Cow;

use greybel_core::{BinaryOp, BitwiseOp};

use super::{Number, Text, Value};

const CONCAT_GUESS: usize = 8;

impl Value {
    #[inline(always)]
    pub(crate) fn apply_numbers(operator: BinaryOp, x: Number, y: Number) -> Self {
        match operator {
            BinaryOp::Add => Self::Number(x + y),
            BinaryOp::Subtract => Self::Number(x - y),
            BinaryOp::Multiply => Self::Number(x * y),
            BinaryOp::Divide => Self::Number(x / y),
            BinaryOp::Modulo => Self::Number(x % y),
            BinaryOp::Power => Self::Number(x.pow(y)),
            BinaryOp::Equal => (x == y).into(),
            BinaryOp::NotEqual => (x != y).into(),
            BinaryOp::Less => (x < y).into(),
            BinaryOp::LessEqual => (x <= y).into(),
            BinaryOp::Greater => (x > y).into(),
            BinaryOp::GreaterEqual => (x >= y).into(),
        }
    }

    /// Evaluates numeric comparison `operator`; arithmetic operators return `None`.
    #[inline(always)]
    pub(crate) fn compare_numbers(operator: BinaryOp, x: Number, y: Number) -> Option<bool> {
        Some(match operator {
            BinaryOp::Equal => x == y,
            BinaryOp::NotEqual => x != y,
            BinaryOp::Less => x < y,
            BinaryOp::LessEqual => x <= y,
            BinaryOp::Greater => x > y,
            BinaryOp::GreaterEqual => x >= y,
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Modulo
            | BinaryOp::Power => return None,
        })
    }

    fn number_or_zero(&self) -> Option<Number> {
        match self {
            Self::Number(number) => Some(*number),
            Self::Null => Some(Number::ZERO),
            _ => None,
        }
    }

    pub(crate) fn arithmetic(
        &self,
        other: &Self,
        apply: impl Fn(Number, Number) -> Number,
    ) -> Self {
        match (self, other.number_or_zero()) {
            (Self::Number(x), Some(y)) => Self::Number(apply(*x, y)),
            _ => Self::Null,
        }
    }

    pub(crate) fn compare(&self, other: &Self, test: impl Fn(std::cmp::Ordering) -> bool) -> Self {
        match (self, other) {
            (Self::Number(x), _) => match other.number_or_zero() {
                Some(y) => x.partial_cmp(&y).is_some_and(test).into(),
                None => Self::Null,
            },
            (Self::String(x), Self::String(y)) => test(mono_string::compare_ordinal(x, y)).into(),
            _ => Self::Null,
        }
    }

    pub(crate) fn bitwise(&self, operator: BitwiseOp, other: &Self) -> Self {
        let (Self::Number(x), Some(y)) = (self, other.number_or_zero()) else {
            return Self::Null;
        };

        Self::Number(x.bitwise(operator, y))
    }

    fn is_object(&self) -> bool {
        matches!(self, Self::List(_) | Self::Map(_) | Self::Function(_))
    }

    pub(crate) fn scalar_binary(operator: BinaryOp, a: &Self, b: &Self) -> Option<Self> {
        if a.is_object() || b.is_object() {
            return None;
        }

        Some(match operator {
            BinaryOp::Add => match (a, b) {
                (Self::String(_), _) | (_, Self::String(_)) => Self::concat(a, b, |_, _| {}),
                _ => a.arithmetic(b, |x, y| x + y),
            },
            BinaryOp::Subtract => match (a, b) {
                (Self::String(text), Self::Null) => Self::String(text.clone()),
                (Self::String(text), Self::String(suffix)) => Self::strip_suffix(text, suffix),
                (Self::String(_), _) => Self::Null,
                _ => a.arithmetic(b, |x, y| x - y),
            },
            BinaryOp::Multiply | BinaryOp::Divide if matches!(a, Self::String(_)) => return None,
            BinaryOp::Multiply => a.arithmetic(b, |x, y| x * y),
            BinaryOp::Divide => a.arithmetic(b, |x, y| x / y),
            BinaryOp::Modulo => a.arithmetic(b, |x, y| x % y),
            BinaryOp::Power => a.arithmetic(b, Number::pow),
            BinaryOp::Equal => a.scalar_equal(b).into(),
            BinaryOp::NotEqual => (!a.scalar_equal(b)).into(),
            BinaryOp::Less => a.compare(b, std::cmp::Ordering::is_lt),
            BinaryOp::LessEqual => a.compare(b, std::cmp::Ordering::is_le),
            BinaryOp::Greater => a.compare(b, std::cmp::Ordering::is_gt),
            BinaryOp::GreaterEqual => a.compare(b, std::cmp::Ordering::is_ge),
        })
    }

    fn scalar_equal(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Number(x), Self::Number(y)) => x == y,
            (Self::String(x), Self::String(y)) => x == y,
            _ => false,
        }
    }

    pub(crate) fn concat(
        a: &Self,
        b: &Self,
        mut write_object: impl FnMut(&mut String, &Self),
    ) -> Self {
        let mut buffers = [itoa::Buffer::new(), itoa::Buffer::new()];
        let [first, second] = &mut buffers;
        let parts = [Part::of(a, first), Part::of(b, second)];

        if let [Some((x, x_ascii)), Some((y, y_ascii))] = parts.each_ref().map(Part::text) {
            return Self::String(Text::concat(&[x, y], x_ascii && y_ascii));
        }

        let mut out = String::with_capacity(parts.iter().map(Part::len).sum());
        let mut ascii = true;

        for part in parts {
            match part {
                Part::Null => {}
                Part::Text(text) => {
                    out.push_str(text);
                    ascii &= text.is_ascii();
                }
                Part::Number(number) => out.push_str(&number),
                Part::Object(object) => {
                    let start = out.len();

                    write_object(&mut out, object);
                    ascii &= out[start..].is_ascii();
                }
            }
        }

        Self::String(Text::with_ascii(&out, ascii))
    }

    pub(crate) fn strip_suffix(text: &Text, suffix: &str) -> Self {
        if !mono_string::collation::ends_with(text, suffix) {
            return Self::String(text.clone());
        }

        let Some(prefix_len) =
            mono_string::utf16::len(text).checked_sub(mono_string::utf16::len(suffix))
        else {
            return Self::String(text.clone());
        };

        Self::from(mono_string::utf16::substring(text, 0..prefix_len).as_ref())
    }
}

enum Part<'v> {
    Null,
    Text(&'v Text),
    Number(Cow<'v, str>),
    Object(&'v Value),
}

impl<'v> Part<'v> {
    fn of(value: &'v Value, buffer: &'v mut itoa::Buffer) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::String(text) => Self::Text(text),
            Value::Number(number) => Self::Number(number.text(buffer)),
            object => Self::Object(object),
        }
    }

    fn text(&self) -> Option<(&str, bool)> {
        match self {
            Self::Null => Some(("", true)),
            Self::Text(text) => Some((text, text.is_ascii())),
            Self::Number(number) => Some((number, true)),
            Self::Object(_) => None,
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Null => 0,
            Self::Text(text) => text.len(),
            Self::Number(number) => number.len(),
            Self::Object(_) => CONCAT_GUESS,
        }
    }
}

#[cfg(test)]
#[path = "tests/ops.rs"]
mod tests;
