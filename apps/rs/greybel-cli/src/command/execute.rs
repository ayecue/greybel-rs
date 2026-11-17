use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::Arc;

use clap::{Args, ValueEnum};
use greybel_documents::Uri;
use greybel_documents_fs_provider::FsProvider;
use greybel_project::{Place, Project};
use greybel_runtime::connect_over_tcp;
use greybel_runtime::protocol::{Exit, Launch, LaunchFailure, SourceBreakpoint};
use greyhack_terminal_text::LineSplitRule;

use super::block_on;
use super::options::{
    MessageHook, ProjectOptions, Simulation, SourceOptions, VariableOptions, current_folder,
};
use crate::report::{self, Failure, Report};
use crate::terminal::{Debugger, RichTextScreen, TerminalHost};

#[derive(Debug, Args)]
pub struct ExecuteCommand {
    #[command(flatten)]
    project: ProjectOptions,
    /// File to run [default: greybel.json's first entry].
    filepath: Option<PathBuf>,
    /// Program's `params` [default: greybel.json's].
    #[arg(short = 'p', long, num_args = 1..)]
    params: Option<Vec<String>>,
    /// Read the program's `params` from the terminal before execution.
    #[arg(short = 'i', long, conflicts_with = "params")]
    interactive: bool,
    /// Enable the terminal debugger and stop at `debugger` statements and breakpoints.
    #[arg(short = 'd', long)]
    debug: bool,
    /// Breakpoint, as `FILE:LINE`; implies `--debug`.
    #[arg(long = "break", value_name = "FILE:LINE")]
    breakpoints: Vec<String>,
    #[command(flatten)]
    simulation: Simulation,
    /// Address of a game server (greyhack-world-server) to run the program in its world, shared with the programs of
    /// other processes, instead of a world generated from the seed.
    #[arg(long, value_name = "ADDRESS", conflicts_with = "seed")]
    world: Option<String>,
    #[command(flatten)]
    variables: VariableOptions,
    #[command(flatten)]
    #[expect(
        dead_code,
        reason = "accepted as greybel-js accepts it: a run reports nothing but its failure"
    )]
    report: Report,
    /// Program execution environment.
    #[arg(
        long = "env-type",
        value_enum,
        ignore_case = true,
        default_value = "mock"
    )]
    env_type: EnvironmentType,
    #[command(flatten)]
    message_hook: MessageHook,
    /// Name of the binary the program runs as, in its home folder [default: myprogram].
    #[arg(long = "program-name", alias = "programName")]
    program_name: Option<String>,
    #[command(flatten)]
    sources: SourceOptions,
}

/// Program execution environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum EnvironmentType {
    /// Use a simulated Grey Hack world.
    Mock,
    /// Use Grey Hack's single-player game through its message hook.
    #[value(alias = "ingame")]
    InGame,
}

impl ExecuteCommand {
    pub fn run(self) -> Result<(), Failure> {
        let (launch, split_rule) = self.launch()?;
        let breakpoints = self.breakpoints().map_err(Failure::input)?;

        report::exit(run(launch, &breakpoints, self.world, split_rule))
    }

    /// The launch and how its output splits long lines.
    fn launch(&self) -> Result<(Launch<'static>, LineSplitRule), Failure> {
        let (mut project, entry) = self.entry()?;
        let manifest = &mut project.manifest;

        self.sources.apply(manifest);
        self.simulation.apply(manifest);

        if let Some(name) = &self.program_name {
            manifest.run.program_name.clone_from(name);
        }

        if self.interactive {
            manifest.run.params = read_params().map_err(Failure::input)?;
        } else if let Some(params) = &self.params {
            manifest.run.params.clone_from(params);
        }

        let place = match self.env_type {
            EnvironmentType::Mock => Place::Simulation,
            EnvironmentType::InGame => Place::Game {
                port: self.message_hook.port,
            },
        };
        let variables = self
            .variables
            .read(&project, &project.manifest.run.variables)?;

        let launch = project.launch(
            &entry,
            place,
            variables,
            self.debug || !self.breakpoints.is_empty(),
        );

        Ok((
            launch,
            RichTextScreen::split_rule_of(project.manifest.run.line_limits.as_ref()),
        ))
    }

    /// File to run and its project: `filepath` in its project, or the first entry of the current folder's project.
    fn entry(&self) -> Result<(Project, Uri), Failure> {
        let Some(filepath) = &self.filepath else {
            let project = self.project.current_project()?;
            let entry = project.entries(&current_folder()).into_iter().next();

            return entry.map(|entry| (project, entry)).ok_or_else(|| {
                Failure::input("Name the file to run, or the entries of greybel.json.")
            });
        };

        if !filepath.is_file() {
            return Err(Failure::input(format!(
                "{} isn't a file.",
                filepath.display()
            )));
        }

        let entry = FsProvider::uri(filepath);

        Ok((self.project.project_of(&entry)?, entry))
    }

    /// Convert `FILE:LINE` options into source breakpoints grouped by file URI.
    fn breakpoints(&self) -> Result<BTreeMap<String, Vec<SourceBreakpoint>>, BreakpointError> {
        let mut breakpoints: BTreeMap<String, Vec<SourceBreakpoint>> = BTreeMap::new();

        for breakpoint in &self.breakpoints {
            let (file, line) = breakpoint
                .rsplit_once(':')
                .and_then(|(file, line)| Some((file, line.parse::<u32>().ok()?)))
                .ok_or_else(|| BreakpointError::Invalid {
                    breakpoint: breakpoint.clone(),
                })?;

            breakpoints
                .entry(FsProvider::uri(Path::new(file)).to_string())
                .or_default()
                .push(SourceBreakpoint {
                    line,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                });
        }

        Ok(breakpoints)
    }
}

/// Execute `launch` on the terminal with `breakpoints` and handle debugger stops as they arrive. Connect to the
/// shared game server at `world` when provided.
fn run(
    launch: Launch<'static>,
    breakpoints: &BTreeMap<String, Vec<SourceBreakpoint>>,
    world: Option<String>,
    split_rule: LineSplitRule,
) -> Result<Exit, LaunchFailure> {
    block_on(async move {
        let world =
            match world {
                Some(address) => Some(connect_over_tcp(&address).await.map_err(|error| {
                    LaunchFailure::Invalid {
                        message: format!("the game server at {address} is unreachable: {error}"),
                        location: None,
                    }
                })?),
                None => None,
            };
        let (controller, mut stops) = TerminalHost::start(Arc::default(), world, split_rule);

        for (uri, breakpoints) in breakpoints {
            controller.set_breakpoints(uri, breakpoints);
        }

        let debugger = Debugger::new(&controller);
        let mut exit = pin!(controller.launch(launch));

        loop {
            tokio::select! {
                exit = &mut exit => return exit,
                Some(notice) = stops.recv() => debugger.serve(&notice).await,
            }
        }
    })
}

/// Invalid breakpoint option error.
#[derive(Debug, thiserror::Error)]
enum BreakpointError {
    #[error("The breakpoint {breakpoint} isn't FILE:LINE.")]
    Invalid { breakpoint: String },
}

/// Program parameter read error.
#[derive(Debug, thiserror::Error)]
enum ParameterError {
    #[error("{0}")]
    Read(#[source] io::Error),
}

/// Read one terminal line and split it into whitespace-separated program parameters.
fn read_params() -> Result<Vec<String>, ParameterError> {
    print!("Params: ");

    let _ = io::stdout().flush();
    let mut line = String::new();

    io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(ParameterError::Read)?;

    Ok(line.split_whitespace().map(ToOwned::to_owned).collect())
}
