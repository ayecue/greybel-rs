//! greyhack-setup: sets up Grey Hack for the message hook. Without arguments it opens a window; with a command it
//! runs on the command line.
#![cfg_attr(all(windows, feature = "window"), windows_subsystem = "windows")]

use std::process::ExitCode;

use clap::Parser;
use greyhack_setup::cli::SetupCli;

#[derive(Debug, Parser)]
#[command(
    name = "greyhack-setup",
    version,
    about = "Sets up Grey Hack for the message hook: BepInEx, the hook plugin and its port. Without a command, opens a window."
)]
struct Cli {
    #[command(flatten)]
    arguments: SetupCli,
}

fn main() -> ExitCode {
    #[cfg(feature = "window")]
    if std::env::args_os().len() <= 1 {
        return greyhack_setup_wizard::window::SetupWindow::open();
    }

    Cli::parse().arguments.run()
}
