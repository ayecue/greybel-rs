use reman::tasks::{Mode, RunOptions};

use crate::support::Fixture;
use crate::support::tasks::{commands, step, workspace};

#[test]
fn should_stop_at_the_first_failure_when_failing_fast() {
    // Arrange
    let fixture = Fixture::new();
    let steps = [step("first", "false"), step("second", "touch second")];
    let options = RunOptions {
        dry_run: false,
        fail_fast: true,
    };

    // Act
    let result = options.run(fixture.root(), &steps);

    // Assert
    assert!(result.is_err() && !fixture.exists("second"));
}

#[test]
fn should_run_every_step_before_failing_without_failing_fast() {
    // Arrange
    let fixture = Fixture::new();
    let steps = [step("first", "false"), step("second", "touch second")];
    let options = RunOptions {
        dry_run: false,
        fail_fast: false,
    };

    // Act
    let result = options.run(fixture.root(), &steps);

    // Assert
    assert!(result.is_err() && fixture.exists("second"));
}

#[test]
fn should_run_a_workspace_task_for_every_package() {
    // Arrange
    let fixture = workspace();

    // Act
    let commands = commands(&fixture.repository(), Mode::Run, "unit", None);

    // Assert
    assert_eq!(
        commands,
        ["rs: nextest --workspace", "rs/app: browser tests"]
    );
}

#[test]
fn should_select_the_affected_packages_of_a_workspace_task() {
    // Arrange
    let fixture = workspace();

    // Act
    let commands = commands(&fixture.repository(), Mode::Run, "unit", Some(&["core"]));

    // Assert
    assert_eq!(
        commands,
        ["rs: nextest -p app -p core", "rs/app: browser tests"]
    );
}

#[test]
fn should_skip_a_project_task_when_the_project_is_unaffected() {
    // Arrange
    let fixture = workspace();

    // Act
    let commands = commands(&fixture.repository(), Mode::Run, "unit", Some(&["other"]));

    // Assert
    assert_eq!(commands, ["rs: nextest -p other"]);
}

#[test]
fn should_run_a_fix_after_the_tasks_it_names() {
    // Arrange
    let fixture = workspace();

    // Act
    let commands = commands(&fixture.repository(), Mode::Fix, "lint", None);

    // Assert
    assert_eq!(commands, ["rs: fmt --all", ": padding --fix"]);
}
