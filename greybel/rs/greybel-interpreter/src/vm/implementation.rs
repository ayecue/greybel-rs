use super::Vm;
use crate::error::{IsaDepthLimit, RuntimeError, RuntimeErrorKind};
use crate::value::{Map, Value};

pub(crate) trait Implementation {
    const ISA_DEPTH: usize;
    const READS_MISSING_SELF: bool;

    fn unindexable(base: &Value) -> Result<(Value, Option<Map>), RuntimeError>;
    fn element_index(key: &Value, list: bool) -> Result<i32, RuntimeError>;
    fn out_of_range(vm: &Vm, base: &Value, key: &Value, index: i32, len: usize) -> RuntimeError;
    fn missing_null_key(vm: &Vm) -> RuntimeError;
    fn isa_loop(limit: IsaDepthLimit) -> Result<(Value, Option<Map>), RuntimeError>;
}

pub(crate) struct GreyHack;

pub(crate) struct MiniScript;

impl Implementation for GreyHack {
    const ISA_DEPTH: usize = 1000;
    const READS_MISSING_SELF: bool = true;

    fn unindexable(_base: &Value) -> Result<(Value, Option<Map>), RuntimeError> {
        Ok((Value::Null, None))
    }

    fn element_index(key: &Value, list: bool) -> Result<i32, RuntimeError> {
        match (key, list) {
            (Value::Null, true) => Err(RuntimeError::from(RuntimeErrorKind::NullListIndex)),
            (Value::Null, false) => Err(RuntimeError::from(RuntimeErrorKind::NullStringIndex)),
            _ => Ok(0),
        }
    }

    fn out_of_range(_vm: &Vm, base: &Value, _key: &Value, index: i32, len: usize) -> RuntimeError {
        let len = i64::try_from(len).unwrap_or(i64::MAX);

        RuntimeError::from(RuntimeErrorKind::IndexOutOfRange {
            kind: base.type_name(),
            index,
            min: -len,
            max: len - 1,
        })
    }

    fn missing_null_key(_vm: &Vm) -> RuntimeError {
        RuntimeError::from(RuntimeErrorKind::NullMapKey)
    }

    fn isa_loop(_limit: IsaDepthLimit) -> Result<(Value, Option<Map>), RuntimeError> {
        Ok((Value::Null, None))
    }
}

impl Implementation for MiniScript {
    const ISA_DEPTH: usize = 256;
    const READS_MISSING_SELF: bool = false;

    fn unindexable(base: &Value) -> Result<(Value, Option<Map>), RuntimeError> {
        let kind = match base {
            Value::Null => RuntimeErrorKind::IndexIntoNull,
            _ => RuntimeErrorKind::NotIndexable,
        };

        Err(RuntimeError::from(kind))
    }

    fn element_index(_key: &Value, list: bool) -> Result<i32, RuntimeError> {
        Err(RuntimeError::from(RuntimeErrorKind::IndexNotNumeric {
            kind: if list { "List" } else { "String" },
        }))
    }

    fn out_of_range(vm: &Vm, base: &Value, key: &Value, _index: i32, _len: usize) -> RuntimeError {
        RuntimeError::from(RuntimeErrorKind::ElementOutOfRange {
            kind: base.type_name(),
            index: vm.heap.display(key).to_string(),
        })
    }

    fn missing_null_key(vm: &Vm) -> RuntimeError {
        vm.missing_key(&Value::Null)
    }

    fn isa_loop(limit: IsaDepthLimit) -> Result<(Value, Option<Map>), RuntimeError> {
        Err(limit.into())
    }
}
