use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::pin::pin;
use std::sync::Arc;

use clap::Args;
use greybel_core::{Dialect, ErrorCode, ParserOptions};
use greybel_documents_fs_provider::FsProvider;
use greybel_project::Place;
use greybel_runtime::protocol::Exit;

use super::block_on;
use super::options::{ProjectOptions, Simulation, SourceOptions, WorldOptions, current_folder};
use crate::report::{self, Failure};
use crate::terminal::{RichTextScreen, TerminalHost};

/// Virtual REPL filename; relative imports use the current directory.
const REPL_FILE: &str = "repl.src";

/// Startup REPL program that hits `debugger` so entered code resumes the same program state.
const REPL_PROGRAM: &str = "debugger";

const PROMPT: &str = "> ";
const CONTINUATION_PROMPT: &str = "... ";

#[derive(Debug, Args)]
pub struct ReplCommand {
    #[command(flatten)]
    project: ProjectOptions,
    #[command(flatten)]
    simulation: Simulation,
    #[command(flatten)]
    sources: SourceOptions,
    #[command(flatten)]
    world: WorldOptions,
}

impl ReplCommand {
    /// Evaluates input in a persistent program until stdin EOF, `exit`, or program termination.
    pub fn run(self) -> Result<(), Failure> {
        let entry = FsProvider::uri(Path::new(REPL_FILE));
        let memory = Arc::new(BTreeMap::from([(entry.clone(), REPL_PROGRAM.to_owned())]));
        let mut project = self.project.current_project()?;

        self.sources.apply(&mut project.manifest);
        self.simulation.apply(&mut project.manifest);

        let variables = project.read_variables(
            &current_folder(),
            &project.manifest.run.variables,
            FsProvider::text,
        );
        let launch = project.launch(&entry, Place::Simulation, variables, true);
        let split_rule = RichTextScreen::split_rule_of(project.manifest.run.line_limits.as_ref());
        let parser = ParserOptions {
            strict_mode: project.manifest.strict_mode,
            syntax_features: project.manifest.syntax.features(),
            ..ParserOptions::new(Dialect::GreyScript)
        };

        let world = self.world.source()?;

        report::exit(block_on(async move {
            let (controller, mut stops) = TerminalHost::start(memory, world, split_rule);
            let mut exit = pin!(controller.launch(launch));

            tokio::select! {
                exit = &mut exit => return exit,
                _ = stops.recv() => {}
            }

            println!("GreyScript REPL: enter code to run it, `exit` to end.");

            while let Some(code) = read_code(&parser) {
                if code == "exit" {
                    break;
                }

                tokio::select! {
                    exit = &mut exit => return exit,
                    evaluation = controller.evaluate(code) => match evaluation {
                        Ok(Some(evaluation)) if evaluation.type_name != "null" => {
                            println!("{}", evaluation.value);
                        }
                        Ok(_) => {}
                        Err(failure) => eprintln!("{failure}"),
                    },
                }
            }

            controller.terminate();

            match exit.await {
                Ok(Exit::Termination) => Ok(Exit::Completion),
                exit => exit,
            }
        }))
    }
}

/// Reads complete code across lines or until an empty line. Returns `None` for EOF before code;
/// otherwise, EOF yields the accumulated code.
fn read_code(parser: &ParserOptions) -> Option<String> {
    let mut code = String::new();

    loop {
        print!(
            "{}",
            if code.is_empty() {
                PROMPT
            } else {
                CONTINUATION_PROMPT
            }
        );

        let _ = io::stdout().flush();
        let mut line = String::new();

        match io::stdin().lock().read_line(&mut line) {
            Ok(0) | Err(_) if code.is_empty() => return None,
            Ok(0) | Err(_) => return Some(code),
            Ok(_) => {}
        }

        let line = line.trim_end_matches(['\n', '\r']);

        if line.trim().is_empty() {
            if code.is_empty() {
                continue;
            }

            return Some(code);
        }

        if !code.is_empty() {
            code.push('\n');
        }

        code.push_str(line);

        if !is_unfinished(&code, parser) {
            return Some(code);
        }
    }
}

/// Checks whether an unclosed block or comment, or an end-of-input parser error, requires another line for `code`.
fn is_unfinished(code: &str, parser: &ParserOptions) -> bool {
    let Err(error) = greybel_core::parse(code, parser.clone()) else {
        return false;
    };

    match error.code() {
        ErrorCode::UnclosedBlock | ErrorCode::UnclosedBlockComment => true,
        ErrorCode::ExpectedToken
        | ErrorCode::ExpectedExpression
        | ErrorCode::ExpectedCondition
        | ErrorCode::UnexpectedToken => error.span.start as usize >= code.trim_end().len(),
        _ => false,
    }
}

#[cfg(test)]
#[path = "tests/repl.rs"]
mod tests;
