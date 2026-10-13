mod cli;

use std::process::ExitCode;

use clap::Parser;
use cli::Command;

fn main() -> ExitCode {
    Command::parse().run()
}
