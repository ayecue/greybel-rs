use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use crate::api::IntoNativeResult;
use crate::api::sequence::SequenceReturn;
use crate::error::RuntimeError;
use crate::value::{Param, Value};
use crate::vm::{ContextEntry, Vm};

pub type NativeFn = Rc<dyn Fn(&mut Vm, &ArgContainer) -> Result<NativeReturn, RuntimeError>>;
pub type NativeFuture = Pin<Box<dyn Future<Output = NativeOutput>>>;
pub type AsyncNativeFn = Rc<dyn Fn(&mut Vm, &ArgContainer) -> NativeFuture>;

pub struct NativeReturn(pub(crate) NativeReturnKind);

pub(crate) enum NativeReturnKind {
    Value(Value),
    Sequence(Box<dyn SequenceReturn>),
    Pending(NativeFuture),
}

impl NativeReturn {
    pub(crate) fn sequence(items: Box<dyn SequenceReturn>) -> Self {
        Self(NativeReturnKind::Sequence(items))
    }

    pub(crate) fn from_future(future: NativeFuture) -> Self {
        Self(NativeReturnKind::Pending(future))
    }
}

impl From<Value> for NativeReturn {
    fn from(value: Value) -> Self {
        Self(NativeReturnKind::Value(value))
    }
}

pub struct NativeOutput(Box<FinishFn>);

type FinishFn = dyn FnOnce(&mut Vm) -> Result<Completion, RuntimeError>;

pub(crate) enum Completion {
    Value(Value),
    Context(ContextEntry),
}

impl NativeOutput {
    pub fn new(result: impl IntoNativeResult + 'static) -> Self {
        Self(Box::new(move |vm| {
            result
                .into_native_result(vm.heap_mut())
                .map(Completion::Value)
        }))
    }

    pub fn with_vm<R: IntoNativeResult>(finish: impl FnOnce(&mut Vm) -> R + 'static) -> Self {
        Self(Box::new(move |vm| {
            let result = finish(vm);

            result
                .into_native_result(vm.heap_mut())
                .map(Completion::Value)
        }))
    }

    pub fn enter(
        entry: impl FnOnce(&mut Vm) -> Result<ContextEntry, RuntimeError> + 'static,
    ) -> Self {
        Self(Box::new(move |vm| entry(vm).map(Completion::Context)))
    }

    pub fn ready(result: impl IntoNativeResult + 'static) -> NativeFuture {
        let output = Self::new(result);

        Box::pin(async move { output })
    }

    pub(crate) fn with_label(self, label: &'static str) -> Self {
        Self(Box::new(move |vm| {
            (self.0)(vm).map_err(|error| error.with_native_label(label))
        }))
    }

    pub(crate) fn finish(self, vm: &mut Vm) -> Result<Completion, RuntimeError> {
        (self.0)(vm).or_else(|error| match error.returned_message() {
            Some(message) => Ok(Completion::Value(Value::from(message))),
            None => Err(error),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ArgContainer {
    pub(crate) params: Rc<[Param]>,
    pub(crate) values: Vec<Value>,
    pub(crate) self_value: Value,
}

impl ArgContainer {
    #[must_use]
    pub fn get(&self, name: &str) -> &Value {
        const NULL: &Value = &Value::Null;

        self.params
            .iter()
            .position(|param| &*param.name == name)
            .map_or(NULL, |index| &self.values[index])
    }

    #[must_use]
    pub fn self_value(&self) -> &Value {
        &self.self_value
    }

    #[must_use]
    pub fn values(&self) -> &[Value] {
        &self.values
    }
}
