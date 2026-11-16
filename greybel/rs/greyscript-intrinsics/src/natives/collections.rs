use greybel_interpreter::{Value, Vm, native};
use greybel_ms_intrinsics::natives::collections as base;

use crate::error::IntrinsicError;

/// Inserts into a list or string. Grey Hack dereferences a null string insertion value and crashes the script.
#[native(method)]
pub fn insert(
    vm: &mut Vm,
    this: &Value,
    index: &Value,
    value: &Value,
) -> Result<Value, IntrinsicError> {
    if matches!(this, Value::String(_)) && value.is_null() {
        // Validate the index first; the game only dereferences the value after its type and range checks.
        base::insert(vm, this, index, &Value::from(""))?;

        return Err(IntrinsicError::GameCrash);
    }

    Ok(base::insert(vm, this, index, value)?)
}

/// C# `ValList.maxSize` limit.
const MAX_LIST_SIZE: usize = 0xFF_FFFF;
const MAX_DELIMITER_LEN: usize = 128;

/// MiniScript-compatible, with list-size and delimiter-size limits.
#[native(method)]
pub fn join(
    vm: &mut Vm,
    this: &Value,
    #[default(" ")] delimiter: &Value,
) -> Result<Value, IntrinsicError> {
    let Value::List(id) = this else {
        return Ok(this.clone());
    };

    if id.items(vm.heap()).len() > MAX_LIST_SIZE {
        return Err(IntrinsicError::JoinTooLarge);
    }

    let delimiter_len = match delimiter {
        Value::String(text) => vm.heap().text_len(text),
        _ => 0,
    };

    if delimiter_len > MAX_DELIMITER_LEN {
        return Err(IntrinsicError::JoinDelimiterTooLarge);
    }

    Ok(base::join(vm, this, delimiter))
}

/// Tests reference identity using C# object comparison semantics.
fn same_object(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::List(a), Value::List(b)) => a == b,
        (Value::Map(a), Value::Map(b)) => a == b,
        (Value::String(a), Value::String(b)) => a.ptr_eq(b),
        _ => false,
    }
}

/// Rejects `method` for game objects and class type maps.
fn refuse_game_object(vm: &Vm, this: &Value, method: &'static str) -> Result<(), IntrinsicError> {
    match this {
        Value::Map(id) if id.is_sealed(vm.heap()) => Err(IntrinsicError::GameObject(method)),
        _ => Ok(()),
    }
}

/// MiniScript-compatible, except self-pushes and game-object targets are forbidden, and map keys cannot be `null`.
#[native(method)]
pub fn push(vm: &mut Vm, this: &Value, value: &Value) -> Result<Value, IntrinsicError> {
    if same_object(this, value) {
        return Err(IntrinsicError::PushSelf);
    }

    refuse_game_object(vm, this, "push")?;
    if matches!(this, Value::Map(_)) && value.is_null() {
        return Err(IntrinsicError::PushNull);
    }

    Ok(base::push(vm, this, value))
}

/// Replaces list or map values; strings instead use their regex-based `replace` method.
#[native(method)]
pub fn replace(
    vm: &mut Vm,
    this: &Value,
    oldval: &Value,
    newval: &Value,
    #[name = "maxCount"] max_count: &Value,
) -> Result<Value, IntrinsicError> {
    if let Value::String(text) = this {
        return Err(IntrinsicError::ReplaceOnString(text.to_string()));
    }

    refuse_game_object(vm, this, "replace")?;

    Ok(base::replace(vm, this, oldval, newval, max_count)?)
}

/// Reverses list elements in place.
#[native(method)]
pub fn reverse(vm: &mut Vm, this: &Value) {
    if let Value::List(id) = this {
        id.items_mut(vm.heap_mut()).reverse();
    }
}

/// MiniScript operation, rejected for game objects.
#[native(method)]
pub fn pop(vm: &mut Vm, this: &Value) -> Result<Value, IntrinsicError> {
    refuse_game_object(vm, this, "pop")?;

    Ok(base::pop(vm, this))
}

/// MiniScript operation, rejected for game objects.
#[native(method)]
pub fn pull(vm: &mut Vm, this: &Value) -> Result<Value, IntrinsicError> {
    refuse_game_object(vm, this, "pull")?;

    Ok(base::pull(vm, this))
}

/// MiniScript operation, rejected for game objects.
#[native(method)]
pub fn remove(vm: &mut Vm, this: &Value, k: &Value) -> Result<Value, IntrinsicError> {
    refuse_game_object(vm, this, "remove")?;

    Ok(base::remove(vm, this, k)?)
}

/// MiniScript operation, rejected for game objects.
#[native(method)]
pub fn shuffle(vm: &mut Vm, this: &Value) -> Result<(), IntrinsicError> {
    refuse_game_object(vm, this, "shuffle")?;
    base::shuffle(vm, this);

    Ok(())
}
