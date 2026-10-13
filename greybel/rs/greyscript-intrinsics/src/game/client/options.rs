//! Parses intrinsic option maps whose keys configure individual options.

use greybel_interpreter::{Heap, Map, Value};

use crate::error::IntrinsicError;

/// Returns an optional options map; rejects every other value type.
pub(in crate::game) fn options(
    value: &Value,
    intrinsic: &'static str,
) -> Result<Option<Map>, IntrinsicError> {
    match value {
        Value::Null => Ok(None),
        Value::Map(map) => Ok(Some(*map)),
        _ => Err(IntrinsicError::InvalidArguments(intrinsic)),
    }
}

/// Returns the value at `key`; `null` counts as unset.
pub(in crate::game) fn option<'h>(
    heap: &'h Heap,
    map: Option<Map>,
    key: &str,
) -> Option<&'h Value> {
    map.and_then(|map| map.get(heap, &Value::from(key)))
        .filter(|value| !value.is_null())
}

/// Returns the string at `key`, if present.
pub(in crate::game) fn string_option(
    heap: &Heap,
    map: Option<Map>,
    key: &str,
    intrinsic: &'static str,
) -> Result<Option<String>, IntrinsicError> {
    option(heap, map, key)
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(IntrinsicError::InvalidArguments(intrinsic))
        })
        .transpose()
}
