use std::borrow::Cow;
use std::collections::BTreeMap;

use super::ValueRef;
use crate::binary::{Bytes, Fixed32, Fixed64};

/// Converts values to the dynamic representation returned by [`FieldAccess::field`](super::FieldAccess::field).
pub trait AsValueRef {
    /// Converts this value to [`ValueRef`] without its static type.
    fn as_value_ref(&self) -> ValueRef<'_>;
}

macro_rules! integers {
    ($($ty:ty),*) => {$(
        impl AsValueRef for $ty {
            fn as_value_ref(&self) -> ValueRef<'_> {
                ValueRef::Integer(i128::from(*self))
            }
        }
    )*};
}

integers!(u8, u16, u32, u64, i8, i16, i32, i64);

impl AsValueRef for Fixed32 {
    fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef::Integer(i128::from(self.0))
    }
}

impl AsValueRef for Fixed64 {
    fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef::Integer(i128::from(self.0))
    }
}

impl AsValueRef for f32 {
    fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef::Float(f64::from(*self))
    }
}

impl AsValueRef for f64 {
    fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef::Float(*self)
    }
}

impl AsValueRef for bool {
    fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef::Bool(*self)
    }
}

impl AsValueRef for char {
    fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef::Char(*self)
    }
}

impl AsValueRef for String {
    fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef::Text(self)
    }
}

impl AsValueRef for Cow<'_, str> {
    fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef::Text(self)
    }
}

impl AsValueRef for Bytes {
    fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef::Bytes(&self.0)
    }
}

impl<T: AsValueRef> AsValueRef for Vec<T> {
    fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef::List(self.iter().map(AsValueRef::as_value_ref).collect())
    }
}

impl<T: AsValueRef + Clone> AsValueRef for Cow<'_, [T]> {
    fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef::List(self.iter().map(AsValueRef::as_value_ref).collect())
    }
}

impl<T: AsValueRef> AsValueRef for BTreeMap<String, T> {
    fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef::Record(
            self.iter()
                .map(|(key, value)| (key.as_str(), value.as_value_ref()))
                .collect(),
        )
    }
}

impl<T: AsValueRef> AsValueRef for Option<T> {
    fn as_value_ref(&self) -> ValueRef<'_> {
        self.as_ref()
            .map_or(ValueRef::Null, AsValueRef::as_value_ref)
    }
}

impl<T: AsValueRef + ?Sized> AsValueRef for Box<T> {
    fn as_value_ref(&self) -> ValueRef<'_> {
        (**self).as_value_ref()
    }
}
