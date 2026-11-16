//! Implements standalone code transpilation for `transform`.

use greybel_builder::protocol::{BuildFailure, Transform};
use greybel_documents_fs_provider::FsProvider;
use greybel_project::ProjectError;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use super::Server;
use super::mode::Mode;
use super::source::{current_folder, failure_text, into_tool_contents, project_of};

#[derive(Debug, thiserror::Error)]
enum TransformError {
    #[error("{source}")]
    CodePath {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("{0}")]
    Project(#[from] ProjectError),
    #[error("{}", failure_text(failure))]
    Transform { failure: BuildFailure },
}

into_tool_contents!(TransformError);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TransformRequest {
    /// GreyScript source code.
    pub code: String,
    /// Output mode: `default` (original formatting without comments), `uglify` (minified), or `beautify`
    /// (formatted). Defaults to the current folder's `greybel.json`.
    pub mode: Option<Mode>,
    /// Controls uglification of names accessed as strings; defaults to the project setting.
    pub obfuscation: Option<bool>,
    /// Names preserved during uglification; defaults to the project setting.
    pub excluded_names: Option<Vec<String>>,
    /// Current-folder `greybel.json` profile for build settings; defaults to base settings.
    pub profile: Option<String>,
}

#[tool_router(router = transform_router, vis = "pub(super)")]
impl Server {
    #[tool(
        description = "Transforms GreyScript code without imports and applies the current folder's `greybel.json` settings. Modes: `default` (remove comments), `uglify` (minify), or `beautify` (format)."
    )]
    async fn transform(
        &self,
        Parameters(params): Parameters<TransformRequest>,
    ) -> Result<String, TransformError> {
        let folder = current_folder();
        let mut project = project_of(
            &folder
                .join_path(&["code.src"])
                .map_err(|error| TransformError::CodePath {
                    source: Box::new(error),
                })?,
            params.profile.as_deref(),
        )?;
        let build = &mut project.manifest.build;

        if let Some(mode) = params.mode {
            build.mode = mode.into();
        }

        if let Some(obfuscation) = params.obfuscation {
            build.obfuscation = obfuscation;
        }

        if let Some(names) = params.excluded_names {
            build.excluded_namespaces = names;
        }

        let variables =
            project.read_variables(&folder, &project.manifest.build.variables, FsProvider::text);

        greybel_builder::transform(&Transform {
            code: params.code.into(),
            transpile: project.transpile_settings(variables),
        })
        .map_err(|failure| TransformError::Transform { failure })
    }
}
