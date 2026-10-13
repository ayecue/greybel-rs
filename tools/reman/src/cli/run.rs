//! Command implementations.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Result, bail};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use reman::check::{Check, Report};
use reman::diff::Diff;
use reman::git::Git;
use reman::init::{InitOptions, Initializer};
use reman::release::plan::Plan;
use reman::release::{Preparation, PublishOptions, Publisher};
use reman::repository::Repository;
use reman::show::{GraphFormat, GraphView, ProjectView};
use reman::tasks::{self, Mode, RunOptions, Selection};

use super::{Cli, Command, ReleaseCommand, TaskOptions};

impl Cli {
    pub fn run(self) -> Result<ExitCode> {
        if let Command::Init {
            kinds,
            dev,
            dry_run,
        } = self.command
        {
            let options = InitOptions {
                date: Self::today(),
                kinds,
                dev,
            };
            let outcome = Initializer::run(&Self::repository_root(&self.root)?, &options)?;

            print!("{}", outcome.report);
            outcome.edits.save(dry_run)?;

            return Ok(ExitCode::SUCCESS);
        }

        self.command.run(&Repository::find(&self.root)?)
    }

    /// Returns the current UTC date.
    fn today() -> Date {
        jiff::Timestamp::now().to_zoned(TimeZone::UTC).date()
    }

    /// Folder of the root `reman.toml`, or the git root when there is none yet.
    fn repository_root(start: &Path) -> Result<PathBuf> {
        match Repository::find(start) {
            Ok(repository) => Ok(repository.root),
            Err(_) => Git::open(start)?.workdir(),
        }
    }
}

impl Command {
    /// Executes any repository command except `init`.
    fn run(self, repository: &Repository) -> Result<ExitCode> {
        match self {
            Command::Init { .. } => unreachable!("`init` runs without a repository"),
            Command::Show { project } => {
                let node = repository.resolve(&project)?;

                print!("{}", ProjectView { repository, node });
            }
            Command::Graph { all, dot } => {
                let format = if dot {
                    GraphFormat::Dot
                } else {
                    GraphFormat::Text
                };

                print!(
                    "{}",
                    GraphView {
                        repository,
                        all,
                        format
                    }
                );
            }
            Command::Impact { since } => {
                for node in Diff::since(repository, since.as_deref())?.impact(repository) {
                    println!("{}", repository.nodes[node].id);
                }
            }
            Command::Change {
                projects,
                message,
                dry_run,
            } => {
                repository
                    .changes()
                    .create(&projects, &message)?
                    .save(dry_run)?;
            }
            Command::Check { since } => {
                let report = Check::run(repository, since.as_deref())?;

                Self::print_report(&report);

                if !report.errors.is_empty() {
                    return Ok(ExitCode::FAILURE);
                }

                println!("ok");
            }
            Command::Plan => print!(
                "{}",
                Plan::from_change_files(repository)?.describe(repository)
            ),
            Command::Release(ReleaseCommand::Prepare {
                dry_run,
                date,
                no_check,
            }) => {
                if !no_check {
                    let report = Check::run(repository, None)?;

                    if !report.errors.is_empty() {
                        Self::print_report(&Report {
                            notes: Vec::new(),
                            ..report
                        });
                        bail!("`reman check` fails (`--no-check` to prepare anyway)");
                    }
                }

                let preparation = Preparation::new(repository, date.unwrap_or_else(Cli::today))?;

                print!("{}", preparation.plan.describe(repository));
                preparation.edits.save(dry_run)?;
            }
            Command::Release(ReleaseCommand::Publish {
                dry_run,
                draft,
                offline,
            }) => {
                let options = PublishOptions {
                    dry_run,
                    draft,
                    offline,
                };
                let publications =
                    Publisher::new(repository, &options)?.publish(&mut std::io::stderr())?;

                println!("{}", serde_json::to_string_pretty(&publications)?);
            }
            Command::Install(options) => {
                options.run(repository, Mode::Run, &[tasks::INSTALL.to_owned()])?
            }
            Command::Lint(options) => {
                options.run(repository, Mode::Run, &[tasks::LINT.to_owned()])?
            }
            Command::Format(options) => {
                options.run(repository, Mode::Fix, &[tasks::LINT.to_owned()])?
            }
            Command::Test { suites, args } => {
                let suites = if suites.is_empty() {
                    tasks::TESTS.map(str::to_owned).to_vec()
                } else {
                    suites
                };

                args.run(repository, Mode::Run, &suites)?;
            }
            Command::Run { suites, args } => args.run(repository, Mode::Run, &suites)?,
        }

        Ok(ExitCode::SUCCESS)
    }

    fn print_report(report: &Report) {
        for note in &report.notes {
            println!("note: {note}");
        }

        for error in &report.errors {
            println!("error: {error}");
        }
    }
}

impl TaskOptions {
    /// Executes `suites` tasks selected by these options.
    fn run(&self, repository: &Repository, mode: Mode, suites: &[String]) -> Result<()> {
        let coverage = match &self.since {
            Some(since) => Some(Diff::since(repository, Some(since))?.impact(repository)),
            None => None,
        };
        let selection = Selection {
            mode,
            suites,
            tasks: &self.tasks,
            languages: &self.languages,
            coverage: coverage.as_ref(),
        };
        let steps = selection.steps(repository)?;

        if steps.is_empty() {
            println!("nothing to run");

            return Ok(());
        }

        let options = RunOptions {
            dry_run: self.dry_run,
            fail_fast: !self.no_fail_fast,
        };

        options.run(&repository.root, &steps)
    }
}
