//! Reference `tests/utils/api.js` test API and helpers for running programs against it.

use std::collections::HashMap;
use std::path::Path;

use greybel_core::{Dialect, ParserOptions};
use greybel_documents::{DocumentOptions, ModuleGraph};
use greybel_documents_fs_provider::FsProvider;
use greybel_documents_memory_provider::MemoryProvider;
use greybel_interpreter::{
    Api, CompileOptions, Interpreter, InterpreterError, InterpreterOptions, IsaDepthLimit, Map,
    PrintOptions, Program, RuntimeError, Scheduler, Value, ValueType, Vm, native, natives,
};

use super::{
    CodeInterpreter, INLINE, constant_clock, inline, interpreter_with_api, run_to_end,
    runtime_error_of, with_captured_stdout,
};
use crate::io::PrintLog;

fn environment() -> HashMap<String, String> {
    HashMap::from([("test".to_owned(), "123".to_owned())])
}

fn error(message: &str) -> RuntimeError {
    RuntimeError::new(message)
}

fn write(vm: &Vm, text: &str) {
    vm.stdout().print(text, PrintOptions::default());
}

#[native]
fn exit(vm: &mut Vm, message: &Value) {
    if let Value::String(_) = message {
        write(vm, &vm.heap().display(message).with_quotes().to_string());
    }

    vm.exit();
}

#[native]
fn print(vm: &mut Vm, value: &Value) {
    write(vm, &vm.heap().display(value).with_quotes().to_string());
}

#[native]
fn stringify(vm: &mut Vm, value: &Value) {
    write(vm, &vm.heap().display(value).to_string());
}

#[native]
fn value_of_test(value: Value) -> Value {
    value
}

#[native]
fn r#typeof(value: &Value) -> &'static str {
    value.type_name()
}

#[native]
fn return_string() -> &'static str {
    "string"
}

#[native]
fn return_nil() {}

#[native]
fn map_to_object(value: Option<Map>) -> Option<Map> {
    value
}

#[native]
fn range(
    #[default(0)] from: f64,
    #[default(0)] to: &Value,
    step: Option<i32>,
) -> Result<Vec<Value>, RuntimeError> {
    let Some(end) = to.as_number() else {
        return Err(error("range() \"to\" parameter not a number"));
    };
    let step = match step.unwrap_or(0) {
        0 if end >= from => 1.0,
        0 => -1.0,
        step => f64::from(step),
    };
    let mut items = Vec::new();
    let mut index = from;

    while if step > 0.0 {
        index <= end
    } else {
        index >= end
    } {
        items.push(Value::from(index));
        index += step;
    }

    Ok(items)
}

#[native]
fn sqrt(#[default(0)] value: Option<f64>) -> Option<f64> {
    value.map(f64::sqrt)
}

#[native(method)]
fn pop(vm: &mut Vm, this: &Value) -> Value {
    match this {
        Value::Map(id) => {
            let first = id
                .entries(vm.heap())
                .get_index(0)
                .map(|(key, value)| (key.clone(), value.clone()));

            match first {
                Some((key, value)) => {
                    id.remove(vm.heap_mut(), &key);

                    value
                }
                None => Value::Null,
            }
        }
        Value::List(id) => id.items_mut(vm.heap_mut()).pop().unwrap_or_default(),
        _ => Value::Null,
    }
}

#[native(method)]
fn remove(
    vm: &mut Vm,
    this: &Value,
    #[name = "keyValue"] key: &Value,
) -> Result<Value, RuntimeError> {
    match this {
        Value::Null => Err(error("argument to 'remove' must not be null")),
        _ if key.is_null() => Err(error("argument to 'remove' must not be null")),
        Value::Map(id) => Ok(id.remove(vm.heap_mut(), key).is_some().into()),
        Value::List(id) => {
            let mut items = id.items_mut(vm.heap_mut());
            let len = i64::try_from(items.len()).unwrap_or_default();
            let mut index = i64::from(key.to_int());

            if index < 0 {
                index += len;
            }

            if (0..len).contains(&index) {
                items.remove(usize::try_from(index).unwrap_or_default());
            }

            Ok(Value::Null)
        }
        Value::String(text) => Ok(text
            .replacen(&*vm.heap().display(key).to_string(), "", 1)
            .into()),
        _ => Err(error("Type Error: 'remove' requires map, list, or string")),
    }
}

#[native(method)]
fn values(vm: &mut Vm, this: &Value) -> Option<Vec<Value>> {
    match this {
        Value::Map(id) => Some(id.entries(vm.heap()).values().cloned().collect()),
        Value::List(id) => Some(id.items(vm.heap()).to_vec()),
        Value::String(text) => Some(text.chars().map(|c| Value::from(c.to_string())).collect()),
        _ => None,
    }
}

/// Uses one untyped method across receiver types to match the reference's `@len == @[].len`. Typed natives may
/// receive collection contents directly, as shown by `push`.
#[native(method)]
fn len(vm: &mut Vm, this: &Value) -> Option<usize> {
    match this {
        Value::String(text) => Some(vm.heap().text_len(text)),
        Value::List(id) => Some(id.items(vm.heap()).len()),
        Value::Map(id) => Some(id.entries(vm.heap()).len()),
        _ => None,
    }
}

#[native(method)]
fn split(vm: &mut Vm, this: &Value, #[default(",")] delimiter: &Value) -> Vec<String> {
    let text = vm.heap().display(this).to_string();
    let delimiter = vm.heap().display(delimiter).to_string();

    if delimiter.is_empty() {
        text.chars().map(String::from).collect()
    } else {
        text.split(delimiter.as_str()).map(str::to_owned).collect()
    }
}

#[native(method)]
fn push(this: &mut Vec<Value>, #[default(",")] value: Value) -> usize {
    this.push(value);

    this.len()
}

#[native(method)]
fn has_index(vm: &mut Vm, this: &Value, value: &Value) -> Result<bool, IsaDepthLimit> {
    let Value::Map(id) = this else {
        return Ok(false);
    };

    Ok(vm.map_lookup(*id, value)?.is_some())
}

/// Natives from reference `tests/utils/api.js`; `print` and `exit` use the VM's [`Stdout`].
pub const TEST_API: Api = Api::new()
    .set_globals(natives![
        exit,
        print,
        stringify,
        "valueOfTest" => value_of_test,
        "typeof" => r#typeof,
        "returnString" => return_string,
        "returnNil" => return_nil,
        "mapToObject" => map_to_object,
        pop,
        remove,
        range,
        sqrt,
        len,
    ])
    .set_intrinsics(ValueType::String, natives![len, split])
    .set_intrinsics(ValueType::List, natives![len, push, pop])
    .set_intrinsics(
        ValueType::Map,
        natives!["hasIndex" => has_index, values, pop],
    );

/// Creates a disk-backed `target` interpreter in `dialect` with the test API and captured output.
pub fn interpreter(target: &str, dialect: Dialect) -> (Interpreter<FsProvider>, PrintLog) {
    let (print_log, options) = with_captured_stdout(InterpreterOptions {
        dialect,
        target: target.to_owned(),
        environment: environment(),
        ..InterpreterOptions::new(constant_clock())
    });

    (
        Interpreter::new(options, FsProvider::default()).with_api(TEST_API),
        print_log,
    )
}

/// Creates an inline test-API interpreter matching the reference `customCode` setup.
pub fn interpreter_for_code(code: &str) -> (CodeInterpreter, PrintLog) {
    interpreter_with(code, InterpreterOptions::new(constant_clock()))
}

/// Creates an inline interpreter using `options` with the test environment and API.
pub fn interpreter_with(code: &str, options: InterpreterOptions) -> (CodeInterpreter, PrintLog) {
    let options = InterpreterOptions {
        environment: environment(),
        ..options
    };

    interpreter_with_api(code, options, TEST_API)
}

/// Compiles inline Greybel `code` to inspect its bytecode.
pub fn program_for_code(code: &str) -> Program {
    let options = DocumentOptions {
        parser: ParserOptions::new(Dialect::Greybel),
        ..DocumentOptions::default()
    };
    let graph =
        ModuleGraph::load(&MemoryProvider::uri(INLINE), &options, &inline(code)).expect("loads");

    let options = CompileOptions {
        dialect: Dialect::Greybel,
        ..CompileOptions::default()
    };

    Program::compile(&graph, &options).expect("compiles")
}

/// Extracts cloned print_log lines and an optional runtime error from an interpreter result.
fn outcome(
    result: Result<(), InterpreterError>,
    print_log: &PrintLog,
) -> (Vec<String>, Option<RuntimeError>) {
    (print_log.borrow().clone(), runtime_error_of(result))
}

/// Runs Greybel program `entry` and returns print_log lines plus an optional runtime error.
pub fn run(entry: &Path) -> (Vec<String>, Option<RuntimeError>) {
    run_as(entry, Dialect::Greybel)
}

/// Runs `entry` in `dialect` and returns print_log lines plus an optional runtime error.
pub fn run_as(entry: &Path, dialect: Dialect) -> (Vec<String>, Option<RuntimeError>) {
    let (mut interpreter, print_log) = interpreter(entry.to_str().expect("utf-8 path"), dialect);

    outcome(run_to_end(&mut interpreter), &print_log)
}

/// Scheduler that runs one instruction per batch and requests garbage collection before every instruction.
struct CollectEveryInstruction;

impl Scheduler for CollectEveryInstruction {
    fn batch_size(&self) -> usize {
        1
    }

    fn should_yield(&mut self, _: usize) -> bool {
        false
    }

    async fn yield_now(&mut self) {}

    fn should_collect(&mut self) -> bool {
        true
    }
}

/// Runs `entry` while requesting garbage collection before every instruction.
pub fn run_with_eager_collection(
    entry: &Path,
    dialect: Dialect,
) -> (Vec<String>, Option<RuntimeError>) {
    let (interpreter, print_log) = interpreter(entry.to_str().expect("utf-8 path"), dialect);
    let mut interpreter = interpreter.with_scheduler(CollectEveryInstruction);

    outcome(run_to_end(&mut interpreter), &print_log)
}

/// Runs inline `code` and returns print_log lines plus an optional runtime error.
pub fn run_code(code: &str) -> (Vec<String>, Option<RuntimeError>) {
    let (mut interpreter, print_log) = interpreter_for_code(code);

    outcome(run_to_end(&mut interpreter), &print_log)
}

/// Runs inline `code` in `dialect` and returns printed lines plus an optional runtime error.
pub fn run_code_as(code: &str, dialect: Dialect) -> (Vec<String>, Option<RuntimeError>) {
    let options = InterpreterOptions {
        dialect,
        ..InterpreterOptions::new(constant_clock())
    };
    let (mut interpreter, print_log) = interpreter_with(code, options);

    outcome(run_to_end(&mut interpreter), &print_log)
}
