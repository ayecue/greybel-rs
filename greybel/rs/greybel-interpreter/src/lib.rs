#![expect(clippy::inline_always, reason = "Hot paths require inlining")]

extern crate self as greybel_interpreter;

pub mod api;
pub mod bytecode;
pub mod compiler;
mod error;
pub mod interpreter;
pub mod value;
pub mod vm;

pub use api::{
    Api, FromArgument, FromArgumentMut, FromVm, IntoNativeResult, Native, NativeDefault,
    NativeParam, NativeWithName, Register, Sequence, SequenceItem,
};
pub use bytecode::{CodeId, Location, Program};
pub use compiler::CompileOptions;
pub use error::{
    CompileError, IsaDepthLimit, NativeError, NativeFailure, RuntimeError, RuntimeErrorKind,
};
pub use greybel_interpreter_macros::native;
pub use greybel_io::{Echo, InputFuture, Key, PrintOptions, Side, Stdin, Stdout};
pub use interpreter::{Interpreter, InterpreterError, InterpreterOptions};
pub use scheduler::{Clock, Sleep};
pub use value::{
    Closure, EncodedText, Function, FunctionBody, Heap, HeapRef, ItemsMut, List, Map, Param, Text,
    TextUnitView, Value, ValueMap,
};
pub use vm::{
    ArgContainer, AsyncNativeFn, ContextEndFn, ContextEntry, ContextExit, DebugControl, FrameView,
    InstructionBudget, NativeFn, NativeFuture, NativeOutput, NativeReturn, Pin, RunResult,
    Scheduler, StepMode, StopReason, TypeTable, ValueType, Vm, YieldHook,
};
