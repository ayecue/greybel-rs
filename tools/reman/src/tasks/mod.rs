//! Runs lint, fix, test, and install tasks declared in `reman.toml`. Repository tasks run once at the root,
//! workspace tasks once per workspace with language and local defaults, and project tasks per project.
//! `--since` selects tasks covering changed packages and dependents; workspace `{packages}` uses that subset.

mod instance;
mod run;

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::Result;
use indexmap::IndexMap;
use instance::Instance;
pub use run::RunOptions;

use crate::language::Language;
use crate::repository::Repository;

/// Suites of `reman install`, `reman lint` and `reman format`, and of `reman test` without suites.
pub const INSTALL: &str = "install";
pub const LINT: &str = "lint";
pub const TESTS: [&str; 2] = ["unit", "integration"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    /// `run` command of tasks of these suites.
    Run,
    /// `fix` command of lints.
    Fix,
}

/// Command to run.
#[derive(Clone, Debug)]
pub struct Step {
    pub label: String,
    pub command: String,
    /// Root-relative path.
    pub cwd: PathBuf,
    pub env: IndexMap<String, String>,
}

/// Task selection for a run.
pub struct Selection<'a> {
    pub mode: Mode,
    pub suites: &'a [String],
    pub tasks: &'a [String],
    pub languages: &'a [&'static dyn Language],
    /// Package coverage filter; None selects all packages.
    pub coverage: Option<&'a BTreeSet<usize>>,
}

impl Selection<'_> {
    /// Steps of the tasks it picks, in declaration order, each after every step of the tasks its `after` names.
    pub fn steps(&self, repository: &Repository) -> Result<Vec<Step>> {
        let mut picks = Vec::new();

        for instance in Instance::all(repository) {
            if let Some(command) = instance.command(self)
                && let Some(step) = instance.step(repository, command, self.coverage)?
            {
                picks.push((instance, step));
            }
        }

        let mut steps = Vec::with_capacity(picks.len());

        while !picks.is_empty() {
            let ready = picks.iter().position(|(instance, _)| {
                !picks
                    .iter()
                    .any(|(other, _)| instance.runs_after(&other.key))
            });
            let (_, step) = picks.remove(ready.unwrap_or(0));

            steps.push(step);
        }

        Ok(steps)
    }
}
