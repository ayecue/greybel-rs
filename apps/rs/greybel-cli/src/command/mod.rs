//! greybel-js-compatible CLI subcommands.

mod build;
mod execute;
mod import;
mod mcp;
mod options;
mod repl;
mod ui;

use std::future::Future;
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser, Subcommand};
use tokio::task::LocalSet;

use crate::report::Failure;

#[derive(Debug, Parser)]
#[command(name = "greybel", version, about = "Tools for GreyScript")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Transpiles GreyScript files with their imports, then writes installers and in-game builds.
    Build(build::BuildCommand),
    /// Copies files into Grey Hack without transformation.
    Import(import::ImportCommand),
    /// Runs a GreyScript program in a simulated world or Grey Hack.
    Execute(execute::ExecuteCommand),
    /// Evaluates entered GreyScript code in a simulated Grey Hack world.
    Repl(repl::ReplCommand),
    /// Launches the web editor.
    Ui(ui::UiCommand),
    /// Exposes greybel tools to models over the Model Context Protocol on stdin/stdout.
    Mcp(mcp::McpCommand),
}

impl Cli {
    /// Runs the selected command and reports failures; without a command, prints help.
    pub fn run(self) -> ExitCode {
        let Some(command) = self.command else {
            return match Self::command().print_help() {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => ExitCode::FAILURE,
            };
        };

        match command.run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(failure) => {
                failure.report();

                ExitCode::FAILURE
            }
        }
    }
}

impl Command {
    fn run(self) -> Result<(), Failure> {
        match self {
            Self::Build(command) => command.run(),
            Self::Import(command) => command.run(),
            Self::Execute(command) => command.run(),
            Self::Repl(command) => command.run(),
            Self::Ui(command) => command.run(),
            Self::Mcp(command) => command.run(),
        }
    }
}

/// Executes `future` on a current-thread Tokio runtime with reactor and timer support for Grey Hack's message hook.
fn block_on<F: Future>(future: F) -> F::Output {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio's runtime builds");

    LocalSet::new().block_on(&runtime, future)
}
