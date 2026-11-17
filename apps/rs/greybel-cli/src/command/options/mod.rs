//! Command-line options shared by multiple commands. Supplied options override the project's `greybel.json`;
//! unspecified settings use its defaults.

mod transpilation;
mod variables;

use std::path::Path;

use clap::Args;
use greybel_documents::Uri;
use greybel_documents_fs_provider::FsProvider;
use greybel_project::{LineLimits, Manifest, Project};
use greyhack_message_hook_client::protocol::DEFAULT_PORT;
pub use transpilation::Transpilation;
pub use variables::VariableOptions;

use crate::report::Failure;

/// Source file reading options: import extensions that may be omitted and syntax-sugar rejection.
#[derive(Debug, Clone, Args)]
pub struct SourceOptions {
    /// File extensions an import may omit, tried in this order [default: src gs ms].
    #[arg(long = "file-extensions", value_name = "EXTENSION", num_args = 1..)]
    pub extensions: Option<Vec<String>>,
    /// Reject syntax sugar that Grey Hack does not support.
    #[arg(long = "strict-mode")]
    pub strict_mode: bool,
}

impl SourceOptions {
    pub fn apply(&self, manifest: &mut Manifest) {
        if let Some(extensions) = &self.extensions {
            manifest.extensions.clone_from(extensions);
        }

        manifest.strict_mode |= self.strict_mode;
    }
}

/// Connection options for Grey Hack's message hook, used by commands to communicate with the game.
#[derive(Debug, Clone, Copy, Args)]
pub struct MessageHook {
    /// Port of Grey Hack's message hook.
    #[arg(long, default_value_t = DEFAULT_PORT)]
    pub port: u16,
}

/// Simulated Grey Hack world programs run in.
#[derive(Debug, Clone, Args)]
pub struct Simulation {
    /// Text the simulated world is generated from: the same seed makes the same world [default: test].
    #[arg(short = 's', long)]
    pub seed: Option<String>,
    /// Cuts printed lines whose visible text is longer than this many characters into pieces of this length, as
    /// the greyhack-customizable-output mod does, instead of the game's rule.
    #[arg(long, value_name = "CHARACTERS", value_parser = clap::value_parser!(u32).range(1..))]
    pub text_per_line: Option<u32>,
    /// With `--text-per-line`: also cuts lines whose tags take more than this many characters.
    #[arg(long, value_name = "CHARACTERS", requires = "text_per_line")]
    pub tags_per_line: Option<u32>,
}

impl Simulation {
    pub fn apply(&self, manifest: &mut Manifest) {
        if let Some(seed) = &self.seed {
            manifest.run.seed.clone_from(seed);
        }

        if let Some(text_per_line) = self.text_per_line {
            manifest.run.line_limits = Some(LineLimits {
                text_per_line,
                tags_per_line: self.tags_per_line,
            });
        }
    }
}

/// Current folder, which files without a `greybel.json` are relative to.
pub fn current_folder() -> Uri {
    FsProvider::uri(Path::new("."))
}

/// Project settings selection: base `greybel.json` settings or one profile's settings.
#[derive(Debug, Clone, Default, Args)]
pub struct ProjectOptions {
    /// Profile of greybel.json whose build and run settings to use.
    #[arg(long, env = "GREYBEL_PROFILE")]
    pub profile: Option<String>,
}

impl ProjectOptions {
    /// Project of a document at `uri`: the nearest `greybel.json` at or above its folder, or the defaults, with
    /// the profile applied.
    pub fn project_of(&self, uri: &Uri) -> Result<Project, Failure> {
        Project::discover(uri, FsProvider::text)
            .and_then(|project| project.with_profile(self.profile.as_deref()))
            .map_err(|error| Failure::new("Project error", error))
    }

    /// Project of the current folder.
    pub fn current_project(&self) -> Result<Project, Failure> {
        let location = current_folder()
            .join_path(&[Project::FILE_NAME])
            .map_err(Failure::input)?;

        self.project_of(&location)
    }
}
