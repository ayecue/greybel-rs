//! Tests `greybel repl` execution in one persistent program.

use crate::support::Project;

#[test]
fn should_run_code_as_it_is_entered_retaining_its_variables() {
    let project = Project::new(&[(
        "lib.src",
        "triple = function(x)\n  return x * 3\nend function",
    )]);

    let run = project.run(
        &["repl"],
        "a = 5\nf = function(x)\n  return x * a\nend function\nf(2)\nimport_code(\"lib.src\")\ntriple(a)\nprint(\"printed\")\nexit\n",
    );
    let stdout = run.stdout();

    assert!(run.success(), "{run:?}");
    assert!(stdout.contains("> 10\n"), "{run:?}");
    assert!(stdout.contains("> 15\n"), "{run:?}");
    assert!(stdout.contains("printed\n"), "{run:?}");
    assert!(!stdout.contains("null"), "{run:?}");
}

#[test]
fn should_report_errors_and_go_on() {
    let project = Project::new(&[]);

    let run = project.run(&["repl"], "missing\n1 + 1\n");

    assert!(run.success(), "{run:?}");
    assert!(run.stderr().contains("missing"), "{run:?}");
    assert!(run.stdout().contains("> 2\n"), "{run:?}");
}
