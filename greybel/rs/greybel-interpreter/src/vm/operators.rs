use std::fmt::Write;

use greybel_core::{BinaryOp, Dialect};

use super::{ValueType, Vm};
use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::value::{List, Number, Text, Value};

impl Vm {
    pub(crate) fn apply_binary(&mut self, operator: BinaryOp, a: &Value, b: &Value) -> Value {
        if let Some(value) = Value::scalar_binary(operator, a, b) {
            return value;
        }

        match operator {
            BinaryOp::Add => self.add_values(a, b),
            BinaryOp::Subtract => a.arithmetic(b, |x, y| x - y),
            BinaryOp::Multiply => match (a, b) {
                (Value::String(text), Value::Number(factor)) => self.repeat_text(text, *factor),
                (Value::List(id), Value::Number(factor)) => self.repeat_list(*id, *factor),
                (Value::String(_), _) => self.mismatch(b, ValueType::Number, "string replication"),
                (Value::List(_), _) => self.mismatch(b, ValueType::Number, "list replication"),
                _ => a.arithmetic(b, |x, y| x * y),
            },
            BinaryOp::Divide => match (a, b) {
                (Value::String(text), Value::Number(divisor)) => {
                    self.repeat_text(text, Number::ONE / *divisor)
                }
                (Value::List(id), Value::Number(divisor)) => {
                    self.repeat_list(*id, Number::ONE / *divisor)
                }
                (Value::String(_), _) => self.mismatch(b, ValueType::Number, "string division"),
                (Value::List(_), _) => self.mismatch(b, ValueType::Number, "list division"),
                _ => a.arithmetic(b, |x, y| x / y),
            },
            BinaryOp::Modulo => a.arithmetic(b, |x, y| x % y),
            BinaryOp::Power => a.arithmetic(b, Number::pow),
            BinaryOp::Equal => self.heap.equal(a, b).into(),
            BinaryOp::NotEqual => (!self.heap.equal(a, b)).into(),
            BinaryOp::Less => a.compare(b, std::cmp::Ordering::is_lt),
            BinaryOp::LessEqual => a.compare(b, std::cmp::Ordering::is_le),
            BinaryOp::Greater => a.compare(b, std::cmp::Ordering::is_gt),
            BinaryOp::GreaterEqual => a.compare(b, std::cmp::Ordering::is_ge),
        }
    }

    fn add_values(&mut self, a: &Value, b: &Value) -> Value {
        match (a, b) {
            (Value::String(_), _) | (_, Value::String(_)) => {
                let heap = &self.heap;

                Value::concat(a, b, |out, object| {
                    let _ = write!(out, "{}", heap.display(object));
                })
            }
            (Value::List(x), Value::List(y)) => {
                let items = [x.items(&self.heap), y.items(&self.heap)].concat();

                Value::List(self.heap.add_list(items))
            }
            (Value::Map(x), Value::Map(y)) => Value::Map(x.merge(&mut self.heap, *y)),
            (Value::List(_), _) => self.mismatch(b, ValueType::List, "list concatenation"),
            (Value::Map(_), _) => self.mismatch(b, ValueType::Map, "map combination"),
            _ => a.arithmetic(b, |x, y| x + y),
        }
    }

    #[cold]
    fn mismatch(&mut self, operand: &Value, kind: ValueType, operation: &'static str) -> Value {
        let class = |kind| match kind {
            ValueType::String => "Miniscript.ValString",
            ValueType::Number => "Miniscript.ValNumber",
            ValueType::List => "Miniscript.ValList",
            ValueType::Map => "Miniscript.ValMap",
            ValueType::Function => "Miniscript.ValFunction",
        };
        let operand = match operand {
            Value::Null => "null",
            Value::String(_) => "a Miniscript.ValString",
            Value::Number(_) => "a Miniscript.ValNumber",
            Value::List(_) => "a Miniscript.ValList",
            Value::Map(_) => "a Miniscript.ValMap",
            Value::Function(_) => "a Miniscript.ValFunction",
        };

        self.raise(RuntimeError::from(RuntimeErrorKind::TypeMismatch {
            operand,
            class: class(kind),
            operation,
        }));

        Value::Null
    }

    fn repeat_text(&mut self, text: &Text, factor: Number) -> Value {
        let units = text.units(self.heap.string_encoding());
        let len = units.len();
        let count = match self.program.dialect {
            Dialect::MiniScript => repeated_text_miniscript(len, factor.get()),
            Dialect::GreyScript | Dialect::Greybel => repeated_text_greyscript(len, factor),
        };

        match count {
            Ok(Some(count)) => Value::String(units.repeat_to(count)),
            Ok(None) => Value::Null,
            Err(kind) => {
                self.raise(RuntimeError::from(kind));

                Value::Null
            }
        }
    }

    fn repeat_list(&mut self, id: List, factor: Number) -> Value {
        let len = id.items(&self.heap).len();
        let count = match self.program.dialect {
            Dialect::MiniScript => repeated_list_miniscript(len, factor.get()),
            Dialect::GreyScript | Dialect::Greybel => repeated_list_greyscript(len, factor),
        };

        match count {
            Ok(Some(count)) => {
                let items = id.items(&self.heap);
                let items = items.iter().cycle().take(count).cloned().collect();

                Value::List(self.heap.add_list(items))
            }
            Ok(None) => Value::Null,
            Err(kind) => {
                self.raise(RuntimeError::from(kind));

                Value::Null
            }
        }
    }

    pub(crate) fn condition(&self, value: &Value) -> Number {
        match value {
            Value::Number(number) => number.clamp_truth(),
            _ => self.truth(value),
        }
    }

    pub(crate) fn fuzzy_and(&self, a: &Value, b: &Value) -> Value {
        Value::Number(self.fuzzy(a).fuzzy_and(self.fuzzy(b)))
    }

    pub(crate) fn fuzzy_or(&self, a: &Value, b: &Value) -> Value {
        Value::Number(self.fuzzy(a).fuzzy_or(self.fuzzy(b)))
    }

    fn fuzzy(&self, value: &Value) -> Number {
        match value {
            Value::Number(number) => *number,
            _ => self.truth(value),
        }
    }

    fn truth(&self, value: &Value) -> Number {
        Number::from(self.heap.is_truthy(value))
    }
}

const MAX_SIZE: i64 = 16_777_215;

fn repeated_text_greyscript(len: usize, factor: Number) -> Result<Option<usize>, RuntimeErrorKind> {
    let repeats = factor.to_int();

    if repeats < 0 {
        return Ok(Some(0));
    }

    let len_int = i32::try_from(len).unwrap_or(i32::MAX);

    if i64::from(repeats.wrapping_mul(len_int)) > MAX_SIZE {
        return Err(RuntimeErrorKind::TooLarge("string"));
    }

    if i64::from(repeats) * i64::from(len_int) > MAX_SIZE {
        return Err(RuntimeErrorKind::GameCrash);
    }

    #[allow(clippy::cast_precision_loss)]
    let extra = Number::new(len as f64 * (factor.get() - f64::from(repeats))).to_int();

    if extra > len_int {
        return Err(RuntimeErrorKind::GameCrash);
    }

    #[allow(clippy::cast_sign_loss)]
    Ok(Some(repeats as usize * len + extra.max(0) as usize))
}

fn repeated_list_greyscript(len: usize, factor: Number) -> Result<Option<usize>, RuntimeErrorKind> {
    if factor.get().is_nan() || factor.get() <= 0.0 {
        return Ok(Some(0));
    }

    #[allow(clippy::cast_precision_loss)]
    let count = Number::new(len as f64 * factor.get()).to_int();

    if i64::from(count) > MAX_SIZE {
        return Err(RuntimeErrorKind::TooLarge("list"));
    }

    usize::try_from(count)
        .map(Some)
        .map_err(|_| RuntimeErrorKind::GameCrash)
}

fn repeated_text_miniscript(len: usize, factor: f64) -> Result<Option<usize>, RuntimeErrorKind> {
    if !factor.is_finite() {
        return Ok(None);
    }

    if factor <= 0.0 {
        return Ok(Some(0));
    }

    #[allow(clippy::cast_possible_truncation)]
    let repeats = factor as i32;
    let len_int = i32::try_from(len).unwrap_or(i32::MAX);

    if i64::from(repeats.wrapping_mul(len_int)) > MAX_SIZE {
        return Err(RuntimeErrorKind::TooLarge("string"));
    }

    if i64::from(repeats) * i64::from(len_int) > MAX_SIZE {
        return Err(RuntimeErrorKind::MemoryLimit);
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let extra = (len as f64 * (factor - f64::from(repeats))) as usize;

    #[allow(clippy::cast_sign_loss)]
    Ok(Some(repeats as usize * len + extra.min(len)))
}

fn repeated_list_miniscript(len: usize, factor: f64) -> Result<Option<usize>, RuntimeErrorKind> {
    if !factor.is_finite() {
        return Ok(None);
    }

    if factor <= 0.0 {
        return Ok(Some(0));
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let count = (len as f64 * factor) as i32;

    if i64::from(count) > MAX_SIZE {
        return Err(RuntimeErrorKind::TooLarge("list"));
    }

    #[allow(clippy::cast_sign_loss)]
    Ok(Some(count.max(0) as usize))
}
