//! Tests `greybel import` through Grey Hack's message hook.

use crate::support::Project;

#[test]
fn should_report_a_message_hook_that_doesnt_respond() {
    let project = Project::new(&[("scripts/a.src", "print(1)")]);

    let run = project.run(&["import", "scripts", "-pt", "1"], "");

    assert!(!run.success(), "{run:?}");
    assert!(run.stderr().contains("port 1"), "{run:?}");
}

#[test]
fn should_report_a_missing_target() {
    let project = Project::new(&[]);

    let run = project.run(&["import", "missing"], "");

    assert!(!run.success(), "{run:?}");
    assert!(
        run.stderr()
            .starts_with("Invalid input: No files in missing."),
        "{run:?}"
    );
}
