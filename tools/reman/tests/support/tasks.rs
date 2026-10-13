use std::path::PathBuf;

use indexmap::IndexMap;
use reman::paths::PathExt;
use reman::repository::Repository;
use reman::tasks::{Mode, Selection, Step};

use super::Fixture;

const ROOT_TASKS: &str = r#"
[tasks.names]
suite = "lint"
run = "names"

[tasks.padding]
suite = "lint"
fix = "padding --fix"
after = ["rustfmt"]

[languages.rust]
select = { all = "--workspace", each = "-p {name}" }

[languages.rust.tasks.rustfmt]
suite = "lint"
run = "fmt {packages} --check"
fix = "fmt {packages}"
select = { all = "--all", each = "-p {name}" }

[languages.rust.tasks.unit]
suite = "unit"
run = "nextest {packages}"
"#;

/// A workspace `rs` where `app` uses `core`, beside `other`; `app` has a task of its own.
pub fn workspace() -> Fixture {
    let fixture = Fixture::new();

    fixture
        .file("reman.toml", &format!("{}{ROOT_TASKS}", super::ROOT_FILE))
        .cargo_workspace("rs")
        .crate_("rs", "core", &[], Some("library"))
        .crate_("rs", "other", &[], Some("library"))
        .crate_("rs", "app", &["core"], None)
        .file("rs/app/reman.toml", "[project]\nkind = \"app\"\n\n[tasks.browser]\nsuite = \"unit\"\nrun = \"browser tests\"\n")
        .released()
        .commit();

    fixture
}

/// A step running `command` at the root.
pub fn step(label: &str, command: &str) -> Step {
    Step {
        label: label.to_owned(),
        command: command.to_owned(),
        cwd: PathBuf::new(),
        env: IndexMap::new(),
    }
}

/// The commands a selection runs, `cwd: command`.
pub fn commands(
    repository: &Repository,
    mode: Mode,
    suite: &str,
    changes: Option<&[&str]>,
) -> Vec<String> {
    let changes = changes.map(|names| {
        let changed = names.iter().map(|name| repository.resolve(name).unwrap());

        repository.with_dependents(
            changed,
            &[reman::language::Scope::Normal, reman::language::Scope::Dev],
        )
    });
    let suites = [suite.to_owned()];
    let selection = Selection {
        mode,
        suites: &suites,
        tasks: &[],
        languages: &[],
        coverage: changes.as_ref(),
    };

    selection
        .steps(repository)
        .unwrap()
        .into_iter()
        .map(|step| format!("{}: {}", step.cwd.to_slash(), step.command))
        .collect()
}
