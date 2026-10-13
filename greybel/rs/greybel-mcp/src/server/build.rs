//! Implements `build` for a file and its imports in the game's execution format.

use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};

use greybel_builder::protocol::BuildFailure;
use greybel_documents_fs_provider::FsProvider;
use greybel_project::ProjectError;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use super::Server;
use super::mode::Mode;
use super::source::{
    SourceError, current_folder, existing_file, failure_text, into_tool_contents, project_of,
};

#[derive(Debug, thiserror::Error)]
enum BuildError {
    #[error("{0}")]
    Source(#[from] SourceError),
    #[error("{0}")]
    Project(#[from] ProjectError),
    #[error("{}", failure_text(failure))]
    Build { failure: BuildFailure },
    #[error("{0}")]
    Write(#[from] WriteError),
}

into_tool_contents!(BuildError);

#[derive(Debug, thiserror::Error)]
enum WriteError {
    #[error("Can't write {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BuildRequest {
    /// Entry file path.
    pub path: String,
    /// Output mode: `default`, `uglify`, or `beautify`; defaults to the project's `greybel.json`.
    pub mode: Option<Mode>,
    /// In-game build folder; defaults to the project setting, `/root/`.
    pub ingame_directory: Option<String>,
    /// Includes installer scripts that recreate the build in-game; defaults to the project setting, enabled.
    pub installer: Option<bool>,
    /// Output folder for built files; when omitted, returns their source instead.
    pub output: Option<String>,
    /// Project `greybel.json` profile for build and run settings; defaults to base settings.
    pub profile: Option<String>,
}

#[tool_router(router = build_router, vis = "pub(super)")]
impl Server {
    #[tool(
        description = "Builds a GreyScript file with the files it imports, as Grey Hack runs them from a folder in the game, with the settings of its project's greybel.json; optionally with installer scripts. Writes the files into an output folder, or returns their code."
    )]
    async fn build(
        &self,
        Parameters(params): Parameters<BuildRequest>,
    ) -> Result<String, BuildError> {
        let entry = existing_file(&params.path)?;
        let mut project = project_of(&entry, params.profile.as_deref())?;
        let build = &mut project.manifest.build;

        if let Some(mode) = params.mode {
            build.mode = mode.into();
        }

        if let Some(directory) = params.ingame_directory {
            build.ingame_directory = directory;
        }

        if let Some(installer) = params.installer {
            build.installer.active = installer;
        }

        let variables = project.read_variables(
            &current_folder(),
            &project.manifest.build.variables,
            FsProvider::text,
        );
        let message = project.build(&[entry], variables, None);
        let result = greybel_builder::build(&message, FsProvider::new)
            .await
            .map_err(|failure| BuildError::Build { failure })?;
        let files = result.files.iter().chain(&result.installer);
        let mut text = String::new();

        match params.output {
            Some(output) => {
                for file in files {
                    let path = Path::new(&output).join(&file.path);

                    write(&path, &file.code)?;

                    let _ = writeln!(text, "wrote {}", path.display());
                }
            }
            None => {
                for file in files {
                    let _ = writeln!(text, "--- {}\n{}\n", file.path, file.code);
                }
            }
        }

        Ok(text)
    }
}

/// Writes `code` to `path` and creates parent folders.
fn write(path: &Path, code: &str) -> Result<(), WriteError> {
    path.parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(path, code))
        .map_err(|source| WriteError::Io {
            path: path.to_owned(),
            source,
        })
}
