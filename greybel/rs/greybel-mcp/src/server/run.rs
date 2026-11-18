//! Implements `run` in a simulated Grey Hack world or in-game when enabled.

mod program;

use std::collections::BTreeMap;
use std::time::Duration;

use greybel_documents_fs_provider::FsProvider;
use greybel_project::{Place, ProjectError};
use greybel_runtime::protocol::{Exit, LaunchFailure};
use program::Program;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use super::Server;
use super::source::{Source, SourceError, current_folder, into_tool_contents, place, project_of};

#[derive(Debug, thiserror::Error)]
enum RunError {
    #[error("{0}")]
    Source(#[from] SourceError),
    #[error("{0}")]
    Project(#[from] ProjectError),
    #[error("This server doesn't run programs in the game.")]
    GameAccess,
    #[error("{}", launch_failure_text(failure))]
    Launch { failure: LaunchFailure },
}

into_tool_contents!(RunError);

/// Default program execution timeout.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum program execution timeout.
const MAX_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunRequest {
    /// GreyScript source to run, with imports resolved from the current folder.
    pub code: Option<String>,
    /// Path of the GreyScript file to run.
    pub path: Option<String>,
    /// Program `params`; defaults to the project values.
    pub params: Option<Vec<String>>,
    /// Ordered program input lines. Exhausted input returns empty lines, matching cancelled game input.
    #[serde(default)]
    pub input: Vec<String>,
    /// Simulated-world seed text; identical seeds produce identical worlds. Defaults to the project setting, `test`.
    pub seed: Option<String>,
    /// Named `#envar` values that override project values.
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    /// Execution timeout in seconds before termination: default 10, maximum 120.
    pub timeout_seconds: Option<f64>,
    /// Uses Grey Hack single-player instead of simulation when the server permits game access.
    #[serde(default)]
    pub in_game: bool,
    /// Project `greybel.json` profile for build and run settings; defaults to base settings.
    pub profile: Option<String>,
}

#[tool_router(router = run_router, vis = "pub(super)")]
impl Server {
    #[tool(
        description = "Runs GreyScript code or a file in a simulated Grey Hack world. Uses preset `params`, preset input, and the project's `greybel.json` settings. Returns the final terminal text and exit status. `get_shell(\"root\", \"test\")` is the local root shell."
    )]
    async fn run(&self, Parameters(params): Parameters<RunRequest>) -> Result<String, RunError> {
        let (entry, memory) = Source::of(params.code, params.path)?.documents();
        let mut project = project_of(&entry, params.profile.as_deref())?;
        let run = &mut project.manifest.run;
        let world = if params.in_game {
            if !self.options.allow_game {
                return Err(RunError::GameAccess);
            }

            Place::Game {
                port: self.options.port,
            }
        } else {
            Place::Simulation
        };

        if let Some(seed) = params.seed {
            run.seed = seed;
        }

        if let Some(program_params) = params.params {
            run.params = program_params;
        }

        let mut variables = project.read_variables(
            &current_folder(),
            &project.manifest.run.variables,
            FsProvider::text,
        );

        variables.extend(params.variables);

        let timeout = params
            .timeout_seconds
            .filter(|seconds| seconds.is_finite() && *seconds > 0.0)
            .map_or(TIMEOUT, Duration::from_secs_f64)
            .min(MAX_TIMEOUT);
        let outcome = Program {
            launch: project.launch(&entry, world, variables, false),
            memory,
            input: params.input,
            timeout,
            split_rule: Program::split_rule_of(project.manifest.run.line_limits.as_ref()),
            world: self.options.world.clone(),
        }
        .run()
        .await;
        let summary = match outcome.exit {
            Ok(Exit::Completion) => "The program completed.".to_owned(),
            Ok(Exit::Termination) if outcome.out_of_time => format!(
                "The program was terminated after {} seconds.",
                timeout.as_secs_f64()
            ),
            Ok(Exit::Termination) => "The program was terminated.".to_owned(),
            Ok(Exit::Crash { message, stack }) => {
                let mut text = format!("The program failed: {message}");

                for location in stack {
                    text.push_str("\n  at ");
                    text.push_str(&place(&location.uri, location.line, location.character));
                }

                text
            }
            Err(failure) => return Err(RunError::Launch { failure }),
        };

        Ok(format!("{summary}\n\nTerminal:\n{}", outcome.screen))
    }
}

/// Program launch failure with an optional source location.
fn launch_failure_text(failure: &LaunchFailure) -> String {
    match failure {
        LaunchFailure::Invalid { location, .. } => match location {
            Some(location) => format!(
                "The program has an error: {failure} at {}",
                place(&location.uri, location.line, location.character)
            ),
            None => format!("The program has an error: {failure}"),
        },
        _ => failure.to_string(),
    }
}
