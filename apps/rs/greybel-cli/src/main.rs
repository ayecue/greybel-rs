mod arguments;
mod command;
mod report;
mod terminal;

use std::process::ExitCode;

use clap::Parser;
use command::Cli;

fn main() -> ExitCode {
    Cli::parse_from(arguments::with_long_names(std::env::args_os())).run()
}
