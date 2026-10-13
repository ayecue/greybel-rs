//! Fixtures for interpreter exits, stack traces, and runtime errors.

use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use greybel_core::PositionEncoding;
use greybel_interpreter::bytecode::Op;
use greybel_interpreter::{
    InterpreterError, InterpreterOptions, Param, RunResult, RuntimeError, Scheduler, Value, Vm,
};
use greybel_test_support::future::block_on;
use greybel_test_support::interpreter::test_api::{
    interpreter_for_code, interpreter_with, program_for_code, run_code,
};
use greybel_test_support::interpreter::{CodeInterpreter, run_to_end};
use greybel_test_support::io::PrintLog;

pub fn error(code: &str) -> String {
    run_code(code).1.expect("fails").to_string()
}

/// Runs `code` and returns its printed lines, panicking if execution fails.
pub fn printed(code: &str) -> Vec<String> {
    let (printed, error) = run_code(code);

    if let Some(error) = error {
        panic!("fails: {error}");
    }

    printed
}

/// Scheduler executing 10 instructions per batch and yielding after each batch.
#[derive(Default)]
pub struct Slices {
    pub yields: usize,
}

impl Scheduler for Slices {
    fn batch_size(&self) -> usize {
        10
    }

    fn should_yield(&mut self, _: usize) -> bool {
        true
    }

    async fn yield_now(&mut self) {
        self.yields += 1;
    }
}

/// Polls `resume()` once and drops the future, leaving the interpreter suspended at its current wait.
pub fn poll_once<S: Scheduler>(
    interpreter: &mut CodeInterpreter<S>,
) -> Poll<Result<RunResult, InterpreterError>> {
    let mut run = std::pin::pin!(interpreter.resume());

    run.as_mut().poll(&mut Context::from_waker(Waker::noop()))
}

/// Shared cell used to complete the `prompt` native in [`prompting`].
type Response = Rc<RefCell<Option<Value>>>;

/// Starts `code` with an async `prompt` native that remains pending until the returned response cell is set.
pub fn prompting(code: &str) -> (CodeInterpreter, PrintLog, Response) {
    let (interpreter, output) = interpreter_for_code(code);
    let response: Response = Rc::default();
    let pending = Rc::clone(&response);
    let mut interpreter = interpreter.with_api(move |vm: &mut Vm| {
        let pending = Rc::clone(&pending);
        let prompt = vm.add_async_native([Param::new("question")], move |_, _| {
            let response = Rc::clone(&pending);

            std::future::poll_fn(move |_| match response.borrow_mut().take() {
                Some(value) => Poll::Ready(Ok(value)),
                None => Poll::Pending,
            })
        });

        vm.define("prompt", prompt);
    });

    block_on(interpreter.start()).expect("starts");

    (interpreter, output, response)
}

/// Function that pauses at a `debugger` statement with `x` and `y` defined.
pub const PAUSED_FRAME: &str = r#"
        f = function(x)
          y = x * 2
          debugger
          print y
        end function
        f(5)
    "#;

/// Allocates a scope, map, and list in each of 50,000 calls.
pub const GARBAGE: &str = r#"
        make = function(n)
          return {"n": n, "items": [n, n + 1]}
        end function
        i = 0
        keep = []
        while i < 50000
          m = make(i)
          if i % 10000 == 0 then keep.push(m)
          i = i + 1
        end while
        print keep[4].items[1]
    "#;

/// Starts `code` and returns its interpreter and captured output.
pub fn started(code: &str) -> (CodeInterpreter, PrintLog) {
    let (mut interpreter, output) = interpreter_for_code(code);

    block_on(interpreter.start()).expect("starts");

    (interpreter, output)
}

/// Builds an interpreter with an async `fetch` native that is pending once before returning 1.
pub fn fetching(code: &str) -> (CodeInterpreter, PrintLog) {
    let (interpreter, output) = interpreter_for_code(code);
    let interpreter = interpreter.with_api(|vm: &mut Vm| {
        let fetch = vm.add_async_native([], |_, _| pending_once(Value::from(1.0)));

        vm.define("fetch", fetch);
    });

    (interpreter, output)
}

/// Builds a program that calls ready and pending async natives, then fails in another async native.
pub fn awaiting() -> (CodeInterpreter, PrintLog) {
    let (interpreter, output) = interpreter_for_code(
        "print now(1) + 1\nprint later(20) + 1\nf = function()\n  return broken\nend function\nprint f",
    );
    let interpreter = interpreter.with_api(|vm: &mut Vm| {
        let now = vm.add_async_native([Param::new("n")], |_, args| {
            let n = args.get("n").to_number();

            async move { Ok(Value::from(n)) }
        });
        let later = vm.add_async_native([Param::new("n")], |_, args| {
            pending_once(Value::from(args.get("n").to_number() * 2.0))
        });
        let broken = vm.add_async_native([], |_, _| async { Err(RuntimeError::new("offline")) });

        vm.define("now", now);
        vm.define("later", later);
        vm.define("broken", broken);
    });

    (interpreter, output)
}

/// Returns a future that wakes itself and stays pending on its first poll, then resolves to `value` on the second.
fn pending_once(
    value: Value,
) -> impl std::future::Future<Output = Result<Value, greybel_interpreter::RuntimeError>> {
    let mut value = Some(value);
    let mut first = true;

    std::future::poll_fn(move |context| {
        if std::mem::take(&mut first) {
            context.waker().wake_by_ref();

            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(Ok(value.take().expect("polled once more")))
        }
    })
}

/// State shared across threads to resolve a native future: an optional value and its waker.
#[derive(Default)]
pub struct Shared {
    pub value: Option<f64>,
    pub waker: Option<Waker>,
}

/// Runs `code` with `string_encoding` set to `encoding` and returns its printed output.
pub fn run_with_encoding(code: &str, encoding: PositionEncoding) -> Vec<String> {
    let options = InterpreterOptions {
        string_encoding: encoding,
        ..InterpreterOptions::new(std::rc::Rc::new(scheduler::ManualClock::constant()))
    };
    let (mut interpreter, output) = interpreter_with(code, options);

    run_to_end(&mut interpreter).expect("runs");

    output.take()
}

/// Scheduler recording instruction counts between yields and yielding after at least 2,048 instructions.
#[derive(Default)]
pub struct Counting {
    pub since_yield: usize,
    pub slices: Vec<usize>,
}

impl Scheduler for Counting {
    fn should_yield(&mut self, executed: usize) -> bool {
        self.since_yield += executed;

        self.since_yield >= 2048
    }

    async fn yield_now(&mut self) {
        self.slices.push(std::mem::take(&mut self.since_yield));
    }
}

/// Compile-time constant expressions, plus runtime string repetition for `"ab" * 2`.
pub const FOLDED: &str =
    "x = 2 * 3 + 1\ny = \"a\" + \"b\" + 1\nz = 10 / 4 - 2 ^ 3\nw = 1 < 2\nv = \"ab\" * 2";

/// Returns whether a function with `body` stores locals in slots rather than a locals map.
pub fn has_slots(body: &str) -> bool {
    let program = program_for_code(&format!("f = function(a)\n{body}\nend function"));
    let entry = program.code(program.entry());
    let template = entry.ops.iter().find_map(|op| match op {
        Op::Function { code, .. } => Some(*code),
        _ => None,
    });

    program.code(template.expect("a function")).slots.is_some()
}

/// Scheduler yielding after every instruction and waking the task so a blocking executor polls again.
pub struct OneAtATime;

impl Scheduler for OneAtATime {
    fn batch_size(&self) -> usize {
        1
    }

    fn should_yield(&mut self, _: usize) -> bool {
        true
    }

    fn yield_now(&mut self) -> impl Future<Output = ()> {
        let mut first = true;

        std::future::poll_fn(move |context| {
            if std::mem::take(&mut first) {
                context.waker().wake_by_ref();

                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
    }
}

/// Drives the started interpreter to a function call, stopping before the callee's first instruction.
pub fn enter_call(interpreter: &mut CodeInterpreter<OneAtATime>) {
    while interpreter.vm().expect("started").stack_trace().len() < 2 {
        assert!(poll_once(interpreter).is_pending());
    }
}
