use std::fmt;
use std::rc::Rc;

use greybel_symbols::names::SELF;

use super::{Key, Map, Value};
use crate::bytecode::CodeId;
use crate::vm::{AsyncNativeFn, NativeFn};

#[derive(Clone)]
pub enum FunctionBody {
    Script(CodeId),
    Native {
        call: NativeFn,
        label: Option<&'static str>,
    },
    AsyncNative {
        call: AsyncNativeFn,
        label: Option<&'static str>,
    },
}

impl FunctionBody {
    pub(crate) fn identity(&self) -> u64 {
        match self {
            Self::Script(code) => u64::from(code.0) << 2,
            Self::Native { call, .. } => (Rc::as_ptr(call).cast::<()>().addr() as u64) << 2 | 1,
            Self::AsyncNative { call, .. } => {
                (Rc::as_ptr(call).cast::<()>().addr() as u64) << 2 | 2
            }
        }
    }
}

impl PartialEq for FunctionBody {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Script(a), Self::Script(b)) => a == b,
            (Self::Native { call: a, .. }, Self::Native { call: b, .. }) => {
                std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
            }
            (Self::AsyncNative { call: a, .. }, Self::AsyncNative { call: b, .. }) => {
                std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
            }
            _ => false,
        }
    }
}

impl Eq for FunctionBody {}

impl fmt::Debug for FunctionBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Script(code) => f.debug_tuple("Script").field(code).finish(),
            Self::Native { label, .. } => f.debug_struct("Native").field("label", label).finish(),
            Self::AsyncNative { label, .. } => {
                f.debug_struct("AsyncNative").field("label", label).finish()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: Rc<str>,
    pub default: Value,
    pub(crate) key: Key,
}

impl Param {
    #[must_use]
    pub fn new(name: impl Into<Rc<str>>) -> Self {
        Self::with_default(name, Value::Null)
    }

    pub fn with_self(params: impl IntoIterator<Item = Self>) -> impl Iterator<Item = Self> {
        std::iter::once(Self::new(SELF)).chain(params)
    }

    #[must_use]
    pub fn with_default(name: impl Into<Rc<str>>, default: Value) -> Self {
        let name = name.into();

        Self {
            key: Key::new(Rc::clone(&name)),
            name,
            default,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub params: Rc<[Param]>,
    pub body: FunctionBody,
    pub outer: Option<Map>,
}
