//! Test native for starting child contexts in `tests/contexts.rs`.

use greybel_interpreter::{
    ArgContainer, ContextEntry, ContextExit, Native, NativeFuture, NativeOutput, NativeParam, Pin,
    Value, Vm,
};
use greybel_test_support::interpreter::test_api::{interpreter_for_code, program_for_code};
use greybel_test_support::interpreter::{CodeInterpreter, run_to_end};
use greybel_test_support::io::PrintLog;

static CODE: &[NativeParam] = &[NativeParam::new("code")];

/// `start(code)` runs `code` in a child context and returns its exit kind, including stack line numbers on failure.
static START: Native = Native::from_async_fn(None, CODE, false, start);

fn start(_vm: &mut Vm, args: &ArgContainer) -> NativeFuture {
    let code = args.get("code").as_str().unwrap_or_default().to_owned();

    Box::pin(async move {
        NativeOutput::enter(move |vm| {
            let entry = vm.add_program(program_for_code(&code));

            Ok(ContextEntry::new(entry, Vec::new(), |_vm, exit| {
                let ending = match exit {
                    ContextExit::Completion => "completion".to_owned(),
                    ContextExit::Termination => "termination".to_owned(),
                    ContextExit::Failure(error) => {
                        let lines: Vec<String> = error
                            .stack
                            .iter()
                            .map(|location| location.line.to_string())
                            .collect();

                        format!("failure at {}", lines.join(" "))
                    }
                };

                NativeOutput::ready(ending)
            }))
        })
    })
}

/// Pinned map returned by `shared` in every context.
struct Shared(
    Value,
    #[expect(dead_code, reason = "only retains the map")] Pin,
);

pub fn register(vm: &mut Vm) {
    let start = vm.native_function(&START);
    let shared = vm.add_native([], |vm, _| {
        if let Some(shared) = vm.host_data::<Shared>() {
            return Ok(shared.0.clone());
        }

        let map = Value::Map(vm.heap_mut().add_map());
        let pin = vm.pin(map.clone());

        vm.set_host_data(Shared(map.clone(), pin));

        Ok(map)
    });

    vm.define("start", start);
    vm.define("shared", shared);
}

pub fn interpreter(code: &str) -> (CodeInterpreter, PrintLog) {
    let (interpreter, printed) = interpreter_for_code(code);

    (interpreter.with_api(register), printed)
}

pub fn printed(code: &str) -> Vec<String> {
    let (mut interpreter, printed) = interpreter(code);

    run_to_end(&mut interpreter).expect("runs");

    printed.take()
}
