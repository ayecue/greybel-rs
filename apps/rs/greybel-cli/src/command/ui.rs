use std::process::Command;

use clap::Args;

use crate::report::Failure;

/// Web editor URL used by default.
const EDITOR_URL: &str = "https://editor.greyscript.org";

#[derive(Debug, Args)]
pub struct UiCommand {
    /// Address of the web editor to open.
    #[arg(long, default_value = EDITOR_URL)]
    url: String,
}

impl UiCommand {
    /// Open the web editor in the system browser.
    pub fn run(self) -> Result<(), Failure> {
        match browser(&self.url).status() {
            Ok(status) if status.success() => Ok(()),
            _ => Err(Failure::new(
                "Can't open a browser",
                format!("the web editor is at {}.", self.url),
            )),
        }
    }
}

/// Construct the platform-specific command that opens `url` in the system browser.
fn browser(url: &str) -> Command {
    if cfg!(target_os = "macos") {
        let mut command = Command::new("open");

        command.arg(url);

        command
    } else if cfg!(windows) {
        let mut command = Command::new("cmd");

        command.args(["/C", "start", "", url]);

        command
    } else {
        let mut command = Command::new("xdg-open");

        command.arg(url);

        command
    }
}
