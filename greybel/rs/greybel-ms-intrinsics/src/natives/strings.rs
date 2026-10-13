use std::borrow::Cow;

use greybel_interpreter::{EncodedText, Text, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::number;

#[native]
pub fn from_code_point(
    #[default(65)]
    #[name = "codePoint"]
    code_point: i32,
) -> Result<String, IntrinsicError> {
    u32::try_from(code_point)
        .ok()
        .and_then(char::from_u32)
        .map(String::from)
        .ok_or(IntrinsicError::InvalidCharCode)
}

#[native(method)]
pub fn code(vm: &mut Vm, this: &Value) -> Result<f64, IntrinsicError> {
    if this.is_null() {
        return Ok(0.0);
    }

    vm.heap()
        .display(this)
        .to_string()
        .chars()
        .next()
        .map(|c| f64::from(u32::from(c)))
        .ok_or(IntrinsicError::EmptyCode)
}

#[native(method)]
pub fn lower(this: Value) -> Value {
    if let Value::String(text) = &this
        && let Cow::Owned(lower_text) = mono_string::to_lower(text)
    {
        return Value::from(lower_text);
    }

    this
}

#[native(method)]
pub fn upper(this: Value) -> Value {
    if let Value::String(text) = &this
        && let Cow::Owned(upper_text) = mono_string::to_upper(text)
    {
        return Value::from(upper_text);
    }

    this
}

#[native(method)]
pub fn split(
    vm: &mut Vm,
    this: &Value,
    #[default(" ")] delimiter: &Value,
    #[default(-1)]
    #[name = "maxCount"]
    max_count: i32,
) -> Vec<Value> {
    let heap = vm.heap();
    let subject = Text::new(heap.display(this).to_string());
    let delimiter = heap.display(delimiter).to_string();
    let limit = usize::try_from(max_count).ok();
    let full = |parts: &[Value]| limit.is_some_and(|limit| parts.len() + 1 == limit);
    let mut parts = Vec::new();

    if delimiter.is_empty() {
        let units = EncodedText::new(&subject, heap.string_encoding());
        let subject_units = heap.text_units(&subject);
        let mut at = 0;

        while let Some(unit) = units.get(at) {
            if full(&parts) {
                parts.push(Value::String(subject_units.slice(at..subject_units.len())));
                break;
            }

            parts.push(Value::from(unit));
            at += 1;
        }

        return parts;
    }

    let mut position = 0;

    while position < subject.len() {
        let next = if full(&parts) {
            subject.len()
        } else {
            subject[position..]
                .find(delimiter.as_str())
                .map_or(subject.len(), |offset| position + offset)
        };

        parts.push(Value::from(&subject[position..next]));
        position = next + delimiter.len();
        if position == subject.len() {
            parts.push(Value::from(""));
        }
    }

    parts
}

#[native]
pub fn to_text(vm: &mut Vm, #[default("")] x: &Value) -> String {
    if x.is_null() {
        String::new()
    } else {
        vm.heap().display(x).to_string()
    }
}

#[native(method)]
pub fn val(#[default(0)] this: Value) -> Value {
    match &this {
        Value::Number(_) => this,
        Value::String(text) => Value::from(number::parse(text)),
        _ => Value::Null,
    }
}
