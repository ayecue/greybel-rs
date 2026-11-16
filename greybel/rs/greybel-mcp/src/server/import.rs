//! Implements `import_into_game` for unchanged files in Grey Hack single-player when game access is enabled.

use std::io;
use std::path::{Path, PathBuf};

use greybel_builder::protocol::{BuildFailure, Import};
use greybel_documents_fs_provider::FsProvider;
use greybel_project::ProjectError;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use super::Server;
use super::source::{failure_text, into_tool_contents, project_of};

#[derive(Debug, thiserror::Error)]
enum ImportError {
    #[error("{source}")]
    AbsolutePath {
        _path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("There are no files at {path}.")]
    NoFile { path: String },
    #[error("{0}")]
    Project(#[from] ProjectError),
    #[error("{}", failure_text(failure))]
    Import { failure: BuildFailure },
}

into_tool_contents!(ImportError);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImportRequest {
    /// File or folder path whose contents are imported unchanged.
    pub path: String,
    /// Destination folder in-game; defaults to the project setting, `/root/`.
    pub ingame_directory: Option<String>,
    /// Project `greybel.json` profile for build and run settings; defaults to base settings.
    pub profile: Option<String>,
}

#[tool_router(router = import_router, vis = "pub(super)")]
impl Server {
    #[tool(
        description = "Imports a file or folder unchanged into Grey Hack's single-player game through the message hook."
    )]
    async fn import_into_game(
        &self,
        Parameters(params): Parameters<ImportRequest>,
    ) -> Result<String, ImportError> {
        let target = std::path::absolute(Path::new(&params.path)).map_err(|source| {
            ImportError::AbsolutePath {
                _path: PathBuf::from(&params.path),
                source,
            }
        })?;
        let files = FsProvider::files(&target);

        if files.is_empty() {
            return Err(ImportError::NoFile { path: params.path });
        }

        let ingame_directory = match params.ingame_directory {
            Some(directory) => directory,
            None => {
                project_of(&files[0], params.profile.as_deref())?
                    .manifest
                    .build
                    .ingame_directory
            }
        };
        let message = Import {
            root: FsProvider::uri(target.parent().unwrap_or(&target))
                .to_string()
                .into(),
            files: files
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .into(),
            ingame_directory: ingame_directory.as_str().into(),
            port: self.options.port,
        };
        let count = greybel_builder::import(&message, FsProvider::new)
            .await
            .map_err(|failure| ImportError::Import { failure })?;

        let noun = if count == 1 { "file" } else { "files" };

        Ok(format!("Imported {count} {noun} into {ingame_directory}."))
    }
}
