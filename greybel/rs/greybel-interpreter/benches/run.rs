//! Benchmarks small programs exercising calls, loops, collections, objects, strings, and closures.

use std::hint::black_box;
use std::io;
use std::path::Path;
use std::rc::Rc;

use criterion::{Criterion, criterion_group, criterion_main};
use greybel_documents::{AsyncResourceProvider, Uri};
use greybel_interpreter::{Interpreter, InterpreterOptions, Param, RunResult, Value, Vm};
use greybel_test_support::future::block_on;
use greybel_test_support::io::PrintLog;

const PROGRAMS: &[&str] = &[
    "fib",
    "loops",
    "collections",
    "objects",
    "strings",
    "closures",
];

/// Resource provider returning the same benchmark source for every read.
struct Source(String);

impl AsyncResourceProvider for Source {
    async fn resolve(&self, target: &str) -> io::Result<Uri> {
        Ok(Uri::memory(target))
    }

    async fn relative_to(&self, _: &Uri, target: &str) -> io::Result<Uri> {
        Ok(Uri::memory(target))
    }

    async fn read(&self, _: &Uri) -> io::Result<Option<String>> {
        Ok(Some(self.0.clone()))
    }
}

type Bench = Interpreter<Source>;

fn source(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("benches/programs/{name}.src"));

    std::fs::read_to_string(path).expect("benchmark program")
}

/// Registers `print` and `range` natives, appending printed values to `printed`.
fn register_natives(vm: &mut Vm, printed: &PrintLog) {
    let sink = Rc::clone(printed);
    let print = vm.add_native([Param::new("value")], move |vm, args| {
        sink.borrow_mut()
            .push(vm.heap().display(args.get("value")).to_string());

        Ok(Value::Null)
    });
    let range = vm.add_native([Param::new("from"), Param::new("to")], |vm, args| {
        let (from, to) = (args.get("from").to_number(), args.get("to").to_number());
        let mut items = Vec::new();
        let mut number = from;

        while number <= to {
            items.push(Value::from(number));
            number += 1.0;
        }

        Ok(Value::List(vm.heap_mut().add_list(items)))
    });

    vm.define("print", print);
    vm.define("range", range);
}

fn interpreter(name: &str, source: &str) -> (Bench, PrintLog) {
    let printed = PrintLog::default();
    let sink = Rc::clone(&printed);
    let options = InterpreterOptions {
        target: name.to_owned(),
        ..InterpreterOptions::new(std::rc::Rc::new(scheduler::ManualClock::constant()))
    };
    let interpreter = Interpreter::new(options, Source(source.to_owned()))
        .with_api(move |vm: &mut Vm| register_natives(vm, &sink));

    (interpreter, printed)
}

/// Runs the compiled program on a fresh VM and returns its captured output.
fn run(interpreter: &mut Bench, printed: &PrintLog) -> Vec<String> {
    assert_eq!(
        block_on(interpreter.run()).expect("runs"),
        RunResult::Complete
    );

    printed.take()
}

fn bench(c: &mut Criterion) {
    let sources: Vec<_> = PROGRAMS.iter().map(|name| (*name, source(name))).collect();

    c.bench_function("compile/all", |b| {
        let mut interpreters: Vec<_> = sources
            .iter()
            .map(|(name, source)| interpreter(name, source).0)
            .collect();

        b.iter(|| {
            for interpreter in &mut interpreters {
                black_box(block_on(interpreter.compile()).expect("compiles"));
            }
        });
    });

    let mut group = c.benchmark_group("run");

    for (name, source) in &sources {
        let (mut interpreter, printed) = interpreter(name, source);

        block_on(interpreter.compile()).expect("compiles");
        group.bench_function(*name, |b| {
            b.iter(|| black_box(run(&mut interpreter, &printed)));
        });
    }

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
