//! Command reporting: progress on stdout unless silenced, and failures on stderr.

use std::fmt::Display;

use clap::Args;
use greybel_builder::protocol::BuildFailure;
use greybel_documents::Uri;
use greybel_documents_fs_provider::FsProvider;
use greybel_runtime::protocol::{Exit, LaunchFailure};

/// Progress reporting enabled.
#[derive(Debug, Clone, Copy, Args)]
pub struct Report {
    /// Suppress progress output and report failures only.
    #[arg(long)]
    silent: bool,
}

impl Report {
    /// Write `message` to stdout when progress reporting is enabled.
    pub fn progress(self, message: impl Display) {
        if !self.silent {
            println!("{message}");
        }
    }
}

/// Command failure, printed to stderr as `label: message`.
#[derive(Debug)]
pub struct Failure {
    label: &'static str,
    message: String,
}

impl Failure {
    pub fn new(label: &'static str, message: impl Display) -> Self {
        Self {
            label,
            message: message.to_string(),
        }
    }

    /// Command given what it can't work with.
    pub fn input(message: impl Display) -> Self {
        Self::new("Invalid input", message)
    }

    /// Failed build, import or transformation, at its location if it has one.
    pub fn build(failure: &BuildFailure) -> Self {
        let location = match failure {
            BuildFailure::Invalid {
                location: Some(location),
                ..
            } => Some(place(&location.uri, location.line, location.character)),
            _ => None,
        };

        Self::new("Build error", with_place(failure, location))
    }

    /// Program that couldn't start, at its location if it has one.
    pub fn launch(failure: &LaunchFailure) -> Self {
        let (label, location) = match failure {
            LaunchFailure::Invalid { location, .. } => (
                "Prepare error",
                location
                    .as_ref()
                    .map(|location| place(&location.uri, location.line, location.character)),
            ),
            LaunchFailure::Unreachable { .. } | LaunchFailure::Unavailable => {
                ("Grey Hack error", None)
            }
            LaunchFailure::Busy => ("Runtime error", None),
        };

        Self::new(label, with_place(failure, location))
    }

    /// Write the failure to stderr.
    pub fn report(&self) {
        eprintln!("{}: {}", self.label, self.message);
    }
}

/// `None` for a completed program; otherwise, the crash, termination, or startup failure.
pub fn exit(exit: Result<Exit, LaunchFailure>) -> Result<(), Failure> {
    match exit {
        Ok(Exit::Completion) => Ok(()),
        Ok(Exit::Crash { message, stack }) => {
            let mut text = message;

            for location in stack {
                text.push_str("\n  at ");
                text.push_str(&place(&location.uri, location.line, location.character));
            }

            Err(Failure::new("Runtime error", text))
        }
        Ok(Exit::Termination) => Err(Failure::new(
            "Terminated",
            "The program was ended before its end.",
        )),
        Err(failure) => Err(Failure::launch(&failure)),
    }
}

/// Source location as `path:line:character`, with the path `FsProvider::display_path` shows.
pub fn place(uri: &str, line: u32, character: u32) -> String {
    let path =
        Uri::parse(uri).map_or_else(|_| uri.to_owned(), |uri| FsProvider::display_path(&uri));

    format!("{path}:{line}:{character}")
}

/// Append ` at ` and the place to `failure` when a place is available.
fn with_place(failure: impl Display, place: Option<String>) -> String {
    match place {
        Some(place) => format!("{failure} at {place}"),
        None => failure.to_string(),
    }
}
