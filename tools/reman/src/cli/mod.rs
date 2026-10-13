//! Command line: its commands and options (here) and what each does (`run`).

mod run;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use jiff::civil::Date;
use reman::init::KindPattern;
use reman::language::{Language, LanguageRegistry};

/// Repository's release manager: projects, versions, change files, changelogs, releases, lints and tests.
#[derive(Parser)]
#[command(name = "reman")]
pub struct Cli {
    /// Folder inside the repository (default: the current one).
    #[arg(long, global = true, default_value = ".")]
    pub root: PathBuf,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Creates missing reman.toml files from reviewable guesses and initializes the release record.
    Init {
        /// `KIND=PATTERN`: assigns a kind to packages selected by ID, name, or folder glob
        /// (repeatable; earliest match wins).
        #[arg(long = "kind")]
        kinds: Vec<KindPattern>,
        /// Marks packages and workspace folders as development-only without [project] (repeatable).
        #[arg(long)]
        dev: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Package's settings, with the file each came from, and its dependencies.
    Show { project: String },
    /// Projects and what they depend on (`--all`: every package and test dependency).
    Graph {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        dot: bool,
    },
    /// Lists packages modified since a commit (default: previous release) and their dependents.
    Impact {
        #[arg(long)]
        since: Option<String>,
    },
    /// Creates a change file, for example: `reman change rs/greybel-core:minor greybel-vs:patch -m "…"`.
    Change {
        /// `project:level`, where level is major, minor, patch, or none.
        #[arg(required = true)]
        projects: Vec<String>,
        #[arg(short, long)]
        message: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Files agree with each other, and every project changed since the last release is named by a change file.
    Check {
        #[arg(long)]
        since: Option<String>,
    },
    /// Preview the next release.
    Plan,
    #[command(subcommand)]
    Release(ReleaseCommand),
    /// Runs the `install` suite for all workspace dependencies.
    Install(TaskOptions),
    /// Executes the `lint` suite.
    Lint(TaskOptions),
    /// Applies lint fixes.
    Format(TaskOptions),
    /// Executes the `unit` and `integration` suites or explicitly selected suites such as `game`.
    Test {
        suites: Vec<String>,
        #[command(flatten)]
        args: TaskOptions,
    },
    /// Executes tasks from arbitrary suites.
    Run {
        #[arg(required = true)]
        suites: Vec<String>,
        #[command(flatten)]
        args: TaskOptions,
    },
}

#[derive(Subcommand)]
pub enum ReleaseCommand {
    /// Updates versions, changelogs, and the release record, then consumes change files.
    Prepare {
        #[arg(long)]
        dry_run: bool,
        /// Date of the changelog sections (default: today, UTC).
        #[arg(long)]
        date: Option<Date>,
        /// Prepares the release despite `reman check` failures.
        #[arg(long)]
        no_check: bool,
    },
    /// Creates missing version tags and GitHub releases for released apps, printing release JSON.
    Publish {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        draft: bool,
        /// Read tags only from the local repository, not GitHub.
        #[arg(long)]
        offline: bool,
    },
}

#[derive(Args)]
pub struct TaskOptions {
    /// Select tasks by key (repeatable).
    #[arg(long = "task")]
    pub tasks: Vec<String>,
    /// Select tasks by language (repeatable).
    #[arg(long = "language", value_parser = LanguageRegistry::by_name)]
    pub languages: Vec<&'static dyn Language>,
    /// Select tasks covering packages changed since a commit (`--since` alone: last release) and their dependents.
    #[arg(long, num_args = 0..=1, default_missing_value = "")]
    pub since: Option<String>,
    /// Displays commands without executing them.
    #[arg(long)]
    pub dry_run: bool,
    /// Executes all tasks and reports failure after completion.
    #[arg(long)]
    pub no_fail_fast: bool,
}
