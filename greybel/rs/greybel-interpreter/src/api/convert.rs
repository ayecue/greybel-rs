use std::hash::BuildHasher;
use std::ops::DerefMut;
use std::rc::Rc;

use greybel_io::{Stdin, Stdout};
use greybel_symbols::types::{FUNCTION, LIST, MAP, NUMBER, STRING};
use indexmap::IndexMap;
use scheduler::Clock;

use crate::error::RuntimeError;
use crate::value::{Function, Heap, ItemsMut, List, Map, Value, ValueMap};
use crate::vm::{NativeFuture, NativeReturn, Vm};

const ANY: &str = "any value";

pub trait FromVm {
    fn from_vm(vm: &Vm) -> Self;
}

impl FromVm for Rc<dyn Stdout> {
    fn from_vm(vm: &Vm) -> Self {
        Rc::clone(vm.stdout())
    }
}

impl FromVm for Rc<dyn Clock> {
    fn from_vm(vm: &Vm) -> Self {
        Rc::clone(vm.clock())
    }
}

impl FromVm for Rc<dyn Stdin> {
    fn from_vm(vm: &Vm) -> Self {
        Rc::clone(vm.stdin())
    }
}

pub trait FromArgument<'v, 'h>: Sized {
    const TYPE_DESCRIPTION: &'static str;

    fn from_argument(value: &'v Value, heap: &'h Heap) -> Option<Self>;
}

pub trait FromArgumentMut<'h> {
    const TYPE_DESCRIPTION: &'static str;

    type Guard: DerefMut<Target = Self>;

    fn from_argument_mut(value: &Value, heap: &'h mut Heap) -> Option<Self::Guard>;
}

impl<'v> FromArgument<'v, '_> for &'v Value {
    const TYPE_DESCRIPTION: &'static str = ANY;

    fn from_argument(value: &'v Value, _heap: &Heap) -> Option<Self> {
        Some(value)
    }
}

impl FromArgument<'_, '_> for Value {
    const TYPE_DESCRIPTION: &'static str = ANY;

    fn from_argument(value: &Value, _heap: &Heap) -> Option<Self> {
        Some(value.clone())
    }
}

impl FromArgument<'_, '_> for f64 {
    const TYPE_DESCRIPTION: &'static str = NUMBER;

    fn from_argument(value: &Value, _heap: &Heap) -> Option<Self> {
        Some(value.as_number().unwrap_or_default())
    }
}

impl FromArgument<'_, '_> for i32 {
    const TYPE_DESCRIPTION: &'static str = NUMBER;

    fn from_argument(value: &Value, _heap: &Heap) -> Option<Self> {
        Some(value.as_int().unwrap_or_default())
    }
}

impl FromArgument<'_, '_> for bool {
    const TYPE_DESCRIPTION: &'static str = ANY;

    fn from_argument(value: &Value, heap: &Heap) -> Option<Self> {
        Some(heap.is_truthy(value))
    }
}

impl<'v> FromArgument<'v, '_> for &'v str {
    const TYPE_DESCRIPTION: &'static str = STRING;

    fn from_argument(value: &'v Value, _heap: &Heap) -> Option<Self> {
        value.as_str()
    }
}

impl FromArgument<'_, '_> for String {
    const TYPE_DESCRIPTION: &'static str = STRING;

    fn from_argument(value: &Value, heap: &Heap) -> Option<Self> {
        <&str>::from_argument(value, heap).map(str::to_owned)
    }
}

impl FromArgument<'_, '_> for List {
    const TYPE_DESCRIPTION: &'static str = LIST;

    fn from_argument(value: &Value, _heap: &Heap) -> Option<Self> {
        match value {
            Value::List(id) => Some(*id),
            _ => None,
        }
    }
}

impl FromArgument<'_, '_> for Map {
    const TYPE_DESCRIPTION: &'static str = MAP;

    fn from_argument(value: &Value, _heap: &Heap) -> Option<Self> {
        match value {
            Value::Map(id) => Some(*id),
            _ => None,
        }
    }
}

impl FromArgument<'_, '_> for Function {
    const TYPE_DESCRIPTION: &'static str = FUNCTION;

    fn from_argument(value: &Value, _heap: &Heap) -> Option<Self> {
        match value {
            Value::Function(id) => Some(*id),
            _ => None,
        }
    }
}

impl<'h> FromArgument<'_, 'h> for &'h [Value] {
    const TYPE_DESCRIPTION: &'static str = LIST;

    fn from_argument(value: &Value, heap: &'h Heap) -> Option<Self> {
        match value {
            Value::List(id) => Some(id.items(heap)),
            _ => None,
        }
    }
}

impl<'h> FromArgument<'_, 'h> for &'h ValueMap {
    const TYPE_DESCRIPTION: &'static str = MAP;

    fn from_argument(value: &Value, heap: &'h Heap) -> Option<Self> {
        match value {
            Value::Map(id) => Some(id.entries(heap)),
            _ => None,
        }
    }
}

impl<'v, 'h, T: FromArgument<'v, 'h>> FromArgument<'v, 'h> for Option<T> {
    const TYPE_DESCRIPTION: &'static str = T::TYPE_DESCRIPTION;

    fn from_argument(value: &'v Value, heap: &'h Heap) -> Option<Self> {
        match value {
            Value::Null => Some(None),
            value => T::from_argument(value, heap).map(Some),
        }
    }
}

impl<'h> FromArgumentMut<'h> for Vec<Value> {
    type Guard = ItemsMut<'h>;

    const TYPE_DESCRIPTION: &'static str = LIST;

    fn from_argument_mut(value: &Value, heap: &'h mut Heap) -> Option<ItemsMut<'h>> {
        match value {
            Value::List(id) => Some(id.items_mut(heap)),
            _ => None,
        }
    }
}

pub trait IntoNativeResult {
    fn into_native_result(self, heap: &mut Heap) -> Result<Value, RuntimeError>;

    fn into_native_return(self, heap: &mut Heap) -> Result<NativeReturn, RuntimeError>
    where
        Self: Sized,
    {
        self.into_native_result(heap).map(NativeReturn::from)
    }
}

macro_rules! into_value {
    ($($ty:ty),*) => {$(
        impl IntoNativeResult for $ty {
            fn into_native_result(self, _heap: &mut Heap) -> Result<Value, RuntimeError> {
                Ok(Value::from(self))
            }
        }
    )*};
}

into_value!(Value, f64, bool, String, &str);

macro_rules! into_number {
    ($($ty:ty),*) => {$(
        impl IntoNativeResult for $ty {
            #[allow(clippy::cast_precision_loss, clippy::cast_lossless)]
            fn into_native_result(self, _heap: &mut Heap) -> Result<Value, RuntimeError> {
                Ok(Value::from(self as f64))
            }
        }
    )*};
}

into_number!(i32, i64, u16, u32, usize);

impl IntoNativeResult for () {
    fn into_native_result(self, _heap: &mut Heap) -> Result<Value, RuntimeError> {
        Ok(Value::Null)
    }
}

impl IntoNativeResult for List {
    fn into_native_result(self, _heap: &mut Heap) -> Result<Value, RuntimeError> {
        Ok(Value::List(self))
    }
}

impl IntoNativeResult for Map {
    fn into_native_result(self, _heap: &mut Heap) -> Result<Value, RuntimeError> {
        Ok(Value::Map(self))
    }
}

impl IntoNativeResult for Function {
    fn into_native_result(self, _heap: &mut Heap) -> Result<Value, RuntimeError> {
        Ok(Value::Function(self))
    }
}

impl<T: IntoNativeResult> IntoNativeResult for Vec<T> {
    fn into_native_result(self, heap: &mut Heap) -> Result<Value, RuntimeError> {
        let mut items = Vec::with_capacity(self.len());

        for item in self {
            items.push(item.into_native_result(heap)?);
        }

        Ok(Value::List(heap.add_list(items)))
    }
}

impl<K: IntoNativeResult, V: IntoNativeResult, S: BuildHasher> IntoNativeResult
    for IndexMap<K, V, S>
{
    fn into_native_result(self, heap: &mut Heap) -> Result<Value, RuntimeError> {
        let map = heap.add_map_with_capacity(self.len());

        for (key, value) in self {
            let key = key.into_native_result(heap)?;
            let value = value.into_native_result(heap)?;

            map.set(heap, key, value);
        }

        Ok(Value::Map(map))
    }
}

impl IntoNativeResult for NativeFuture {
    fn into_native_result(self, _heap: &mut Heap) -> Result<Value, RuntimeError> {
        Err(RuntimeError::new(
            "only a native that runs synchronously can return a future",
        ))
    }

    fn into_native_return(self, _heap: &mut Heap) -> Result<NativeReturn, RuntimeError> {
        Ok(NativeReturn::from_future(self))
    }
}

impl<T: IntoNativeResult> IntoNativeResult for Option<T> {
    fn into_native_result(self, heap: &mut Heap) -> Result<Value, RuntimeError> {
        self.map_or(Ok(Value::Null), |value| value.into_native_result(heap))
    }

    fn into_native_return(self, heap: &mut Heap) -> Result<NativeReturn, RuntimeError> {
        self.map_or(Ok(NativeReturn::from(Value::Null)), |value| {
            value.into_native_return(heap)
        })
    }
}

impl<T: IntoNativeResult, E: Into<RuntimeError>> IntoNativeResult for Result<T, E> {
    fn into_native_result(self, heap: &mut Heap) -> Result<Value, RuntimeError> {
        self.map_err(Into::into)
            .and_then(|value| value.into_native_result(heap))
    }

    fn into_native_return(self, heap: &mut Heap) -> Result<NativeReturn, RuntimeError> {
        self.map_err(Into::into)
            .and_then(|value| value.into_native_return(heap))
    }
}
