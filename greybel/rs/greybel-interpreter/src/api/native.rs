use std::rc::Rc;

use greybel_symbols::names::SELF;

use crate::error::RuntimeError;
use crate::value::{FunctionBody, Param, Value};
use crate::vm::{ArgContainer, NativeFuture, NativeReturn, Vm};

pub type NativeCall = fn(&mut Vm, &ArgContainer) -> Result<NativeReturn, RuntimeError>;
pub type AsyncNativeCall = fn(&mut Vm, &ArgContainer) -> NativeFuture;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NativeDefault {
    Null,
    Number(f64),
    String(&'static str),
}

impl NativeDefault {
    #[must_use]
    pub fn to_value(self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Number(number) => Value::from(number),
            Self::String(text) => Value::from(text),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeParam {
    name: &'static str,
    default: NativeDefault,
}

impl NativeParam {
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self::with_default(name, NativeDefault::Null)
    }

    #[must_use]
    pub const fn with_default(name: &'static str, default: NativeDefault) -> Self {
        Self { name, default }
    }

    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    #[must_use]
    pub const fn default(&self) -> NativeDefault {
        self.default
    }

    fn to_param(self) -> Param {
        Param::with_default(self.name, self.default.to_value())
    }
}

#[derive(Debug, Clone, Copy)]
enum Call {
    Sync(NativeCall),
    Async(AsyncNativeCall),
}

#[derive(Debug, Clone, Copy)]
pub struct Native {
    label: Option<&'static str>,
    params: &'static [NativeParam],
    method: bool,
    call: Call,
}

impl Native {
    #[must_use]
    pub const fn from_fn(
        label: Option<&'static str>,
        params: &'static [NativeParam],
        method: bool,
        call: NativeCall,
    ) -> Self {
        Self {
            label,
            params,
            method,
            call: Call::Sync(call),
        }
    }

    #[must_use]
    pub const fn from_async_fn(
        label: Option<&'static str>,
        params: &'static [NativeParam],
        method: bool,
        call: AsyncNativeCall,
    ) -> Self {
        Self {
            label,
            params,
            method,
            call: Call::Async(call),
        }
    }

    #[must_use]
    pub const fn label(&self) -> Option<&'static str> {
        self.label
    }

    #[must_use]
    pub const fn params(&self) -> &'static [NativeParam] {
        self.params
    }

    #[must_use]
    pub const fn is_method(&self) -> bool {
        self.method
    }

    #[must_use]
    pub const fn is_async(&self) -> bool {
        matches!(self.call, Call::Async(_))
    }

    pub(crate) fn add_function(&self, vm: &mut Vm) -> Value {
        let params = self.params.iter().map(|param| param.to_param());
        let has_receiver = self.params.first().is_some_and(|param| param.name == SELF);
        let params: Vec<Param> = if self.method && !has_receiver {
            Param::with_self(params).collect()
        } else {
            params.collect()
        };
        let label = self.label;
        let body = match self.call {
            Call::Sync(call) => FunctionBody::Native {
                call: Rc::new(call),
                label,
            },
            Call::Async(call) => FunctionBody::AsyncNative {
                call: Rc::new(call),
                label,
            },
        };

        vm.add_native_function(params, body)
    }
}
