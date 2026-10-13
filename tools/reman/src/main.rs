mod cli;

use std::process::ExitCode;

use clap::Parser;
use cli::Cli;

fn main() -> ExitCode {
    match Cli::parse().run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");

            ExitCode::FAILURE
        }
    }
}
