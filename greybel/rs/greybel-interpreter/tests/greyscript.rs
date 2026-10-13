//! Snapshots GreyScript fixtures and debugger cases; update with `INSTA_UPDATE=always cargo test`.

use greybel_core::Dialect;
use greybel_interpreter::RunResult;
use greybel_test_support::future::block_on;
use greybel_test_support::interpreter::test_api::{run_as, run_with_eager_collection};
use greybel_test_support::interpreter::{resume_to_end, run_to_end};

use crate::support::greyscript::{MAIN, interpreter};
use crate::support::{pause, snapshot};

#[test]
fn should_pause_at_the_debugger_before_importing_code() {
    let (mut interpreter, _) = interpreter(&[
        (MAIN, "x = 1\ndebugger\nprint x + libValue"),
        ("lib", "libValue = 41"),
    ]);

    let result = pause(&mut interpreter);

    assert!(matches!(result, RunResult::Breakpoint(_)));
}

/// Code typed into the debugger console is GreyScript too; its `import_code` runs in the paused program's scope.
#[test]
fn should_import_code_from_injected_code() {
    let (mut interpreter, output) = interpreter(&[
        (MAIN, "x = 1\ndebugger\nprint x + libValue"),
        ("lib", "libValue = 41"),
    ]);

    pause(&mut interpreter);
    block_on(interpreter.inject("import_code(\"./lib\")\nprint libValue")).expect("injects");

    resume_to_end(&mut interpreter).expect("runs");

    assert_eq!(*output.borrow(), ["41", "42"]);
}

/// A map with a `classID` prints as it, wherever it appears; other maps print their entries.
#[test]
fn should_label_maps_by_their_class_id() {
    let (mut interpreter, output) = interpreter(&[(
        MAIN,
        "a = {\"classID\": \"Foo\"}\nprint [a, {\"inner\": a}]\nprint {\"plain\": 1}\nprint a",
    )]);

    run_to_end(&mut interpreter).expect("runs");
    assert_eq!(
        *output.borrow(),
        ["[Foo, {\"inner\": Foo}]", "{\"plain\": 1}", "Foo"]
    );
}

#[test]
fn should_run_every_script_fixture() {
    insta::glob!("fixtures/greyscript/scripts/*.src", |path| {
        let dialect = Dialect::GreyScript;
        let (printed, error) = run_as(path, dialect);
        let snapshot = snapshot(&printed, error);
        insta::assert_snapshot!(snapshot);
    });
}

/// Output is unchanged when the collector runs after every instruction; `fib.src` makes too many calls.
#[test]
fn should_print_the_same_when_collecting_after_every_instruction() {
    insta::glob!("fixtures/greyscript/scripts/*.src", |path| {
        if path.ends_with("fib.src") {
            return;
        }
        let expected = run_as(path, Dialect::GreyScript);
        let collected = run_with_eager_collection(path, Dialect::GreyScript);
        assert_eq!(collected, expected, "{}", path.display());
    });
}
