//! Example `#[native]` declarations exposed as `Api` constants, with code-execution helpers.

use std::cell::Cell;
use std::rc::Rc;

use greybel_interpreter::{
    Api, InterpreterError, NativeFuture, NativeOutput, PrintOptions, RuntimeError, Sequence, Stdin,
    Stdout, Value, ValueMap, ValueType, Vm, native, natives,
};

use super::test_api::interpreter_for_code;
use super::{run_to_end, runtime_error};
use crate::io::PrintLog;

/// Appends the rendered argument to the host [`PrintLog`], if available.
#[native]
pub fn log(vm: &mut Vm, value: &Value) {
    let text = vm.heap().display(value).to_string();

    if let Some(log) = vm.host_data::<PrintLog>() {
        log.borrow_mut().push(text);
    }
}

/// Computes `a + b`, defaulting `b` to 10.
#[native]
pub fn add(a: f64, #[default(10)] b: f64) -> f64 {
    a + b
}

#[native(label = "greet")]
pub fn greet(#[name = "who"] name: &str, #[default("!")] suffix: &str) -> String {
    format!("hi {name}{suffix}")
}

#[native]
pub fn maybe(value: Option<f64>) -> Option<f64> {
    value.map(|value| value * 2.0)
}

#[native]
pub fn fail(message: String) -> Result<(), RuntimeError> {
    Err(RuntimeError::new(message))
}

#[native(method)]
pub fn size(this: &ValueMap) -> usize {
    this.len()
}

#[native(method)]
pub fn first(this: &[Value], #[default(-1)] fallback: f64) -> Value {
    this.first().cloned().unwrap_or(fallback.into())
}

#[native]
pub async fn later(value: f64) -> f64 {
    value + 1.0
}

thread_local! {
    static YIELDED: Cell<usize> = const { Cell::new(0) };
}

/// Returns and clears the thread-local `SquareIterator` yield count.
pub fn take_yield_count() -> usize {
    YIELDED.with(|yield_count| yield_count.replace(0))
}

/// Iterates over squares of `0..count` and increments a thread-local counter per item.
#[derive(Clone)]
pub struct SquareIterator {
    next: u32,
    count: u32,
}

impl Iterator for SquareIterator {
    type Item = f64;

    fn next(&mut self) -> Option<f64> {
        if self.next >= self.count {
            return None;
        }

        YIELDED.with(|yield_count| yield_count.set(yield_count.get() + 1));

        let next = self.next;

        self.next += 1;

        Some(f64::from(next * next))
    }
}

impl SquareIterator {
    fn sequence(count: i32) -> Sequence<Self> {
        let count = u32::try_from(count).unwrap_or(0);

        Sequence::new(Self { next: 0, count })
    }
}

#[native]
pub fn squares(count: i32) -> Sequence<SquareIterator> {
    SquareIterator::sequence(count)
}

#[native]
pub async fn async_squares(count: i32) -> Sequence<SquareIterator> {
    SquareIterator::sequence(count)
}

/// Base API that [`API`] includes and partially overrides.
pub const BASE: Api = Api::new().set_globals(natives![log, add, "async" => later]);

pub const API: Api = Api::new()
    .set_includes(&[BASE])
    .set_globals(natives![
        "greetUser" => greet,
        maybe,
        fail,
        squares,
        "asyncSquares" => async_squares
    ])
    .set_intrinsics(ValueType::Map, natives![size])
    .set_intrinsics(ValueType::List, natives![first]);

/// Runs `code` under [`API`] and returns logged lines with the interpreter result.
pub fn run_and_log(code: &str) -> (Vec<String>, Result<(), InterpreterError>) {
    run_and_log_with(code, API)
}

/// Runs `code` under `api` and returns logged lines with the interpreter result.
pub fn run_and_log_with(code: &str, api: Api) -> (Vec<String>, Result<(), InterpreterError>) {
    let log = PrintLog::default();
    let (interpreter, _) = interpreter_for_code(code);
    let mut interpreter = interpreter.with_host_data(Rc::clone(&log)).with_api(api);
    let result = run_to_end(&mut interpreter);

    (log.take(), result)
}

/// Runs `code` under [`API`] and returns logged lines; failures panic.
pub fn log_of(code: &str) -> Vec<String> {
    let (log, result) = run_and_log(code);

    result.expect("runs");

    log
}

/// Returns the display message for the error expected from `code` under [`API`].
pub fn error_message(code: &str) -> String {
    run_and_log(code).1.expect_err("fails").to_string()
}

/// Returns the `RuntimeError` expected from `code` under [`API`].
pub fn failure(code: &str) -> RuntimeError {
    runtime_error(run_and_log(code).1.expect_err("fails"))
}

/// Async native that reads one line from host stdin.
#[native]
pub async fn user_input(
    #[host] stdin: Rc<dyn Stdin>,
    #[default("")] prompt: String,
    #[default(false)] masked: bool,
) -> Result<String, RuntimeError> {
    let echo = if masked {
        greybel_interpreter::Echo::Masked
    } else {
        greybel_interpreter::Echo::Visible
    };

    stdin
        .read_line(&prompt, echo)
        .await
        .map_err(|error| RuntimeError::new(error.to_string()))
}

#[native]
pub async fn user_key(#[host] stdin: Rc<dyn Stdin>) -> Result<String, RuntimeError> {
    stdin
        .read_key("")
        .await
        .map(|key| format!("{key:?}"))
        .map_err(|error| RuntimeError::new(error.to_string()))
}

/// Writes to host stdout without a trailing newline.
#[native]
pub fn write(#[host] stdout: Rc<dyn Stdout>, text: &str) {
    stdout.print(
        text,
        PrintOptions {
            new_line: false,
            replace: false,
        },
    );
}

pub const IO: Api =
    Api::new().set_globals(natives!["userInput" => user_input, "userKey" => user_key, write]);

#[native]
pub fn request_yield(vm: &mut Vm) {
    vm.request_yield();
}

#[native]
pub fn truthy(value: bool) -> bool {
    value
}

#[native]
pub fn pairs(count: i32) -> indexmap::IndexMap<String, Vec<i32>> {
    (0..count)
        .map(|index| {
            (
                format!("k{index}"),
                vec![index; usize::try_from(index).unwrap_or(0)],
            )
        })
        .collect()
}

#[native(method)]
pub fn append(this: &mut Vec<Value>, value: Value) -> usize {
    this.push(value);

    this.len()
}

#[native]
pub async fn async_list(count: i32) -> Vec<i32> {
    (0..count).collect()
}

/// Returns a `NativeFuture` whose result, made on the VM, ends the program (`Vm::exit`) and is `1`.
#[native]
pub fn deferred_exit() -> NativeFuture {
    Box::pin(async move {
        NativeOutput::with_vm(move |vm: &mut Vm| {
            vm.exit();

            Value::from(1.0)
        })
    })
}

/// Captures VM stack depth synchronously and returns a `NativeFuture` that creates `[value, depth]` on the VM
/// when resolved.
#[native]
pub fn deferred(vm: &mut Vm, value: f64) -> NativeFuture {
    let stack = vm.stack_trace().len();

    Box::pin(async move {
        NativeOutput::with_vm(move |vm: &mut Vm| {
            #[allow(clippy::cast_precision_loss)]
            let items = vec![Value::from(value), Value::from(stack as f64)];

            Value::List(vm.heap_mut().add_list(items))
        })
    })
}
