//! Implements `check` using the language server's syntax and type-analysis diagnostics.

use std::sync::Arc;

use greybel_documents::{DocumentError, PathResolution, Uri, Workspace};
use greybel_documents_fs_provider::FsProvider;
use greybel_project::ProjectError;
use greybel_type_analyzer::{Analyzer, Database, Diagnostic, DiagnosticOptions, Dialect, Severity};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use super::Server;
use super::source::{Source, SourceError, into_tool_contents, place, project_of};

#[derive(Debug, thiserror::Error)]
enum DiagnosticError {
    #[error("{0}")]
    Source(#[from] SourceError),
    #[error("{0}")]
    Project(#[from] ProjectError),
    #[error("{0}")]
    Document(#[from] DocumentError),
}

into_tool_contents!(DiagnosticError);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckRequest {
    /// GreyScript source with imports resolved from the current folder.
    pub code: Option<String>,
    /// GreyScript file path; imported files are checked too.
    pub path: Option<String>,
}

/// Source workspace and checked files: entry first, followed by imports sorted by URI.
struct Check {
    workspace: Workspace<Database>,
    files: Vec<Uri>,
    options: DiagnosticOptions,
}

/// Diagnostic with its containing document.
struct Problem {
    uri: Uri,
    diagnostic: Diagnostic,
}

#[tool_router(router = check_router, vis = "pub(super)")]
impl Server {
    #[tool(
        description = "Checks GreyScript code, or a file with the files it imports, for syntax errors and what the type analyzer finds, as the language server shows them."
    )]
    async fn check(
        &self,
        Parameters(params): Parameters<CheckRequest>,
    ) -> Result<String, DiagnosticError> {
        let problems = Check::new(Source::of(params.code, params.path)?)?.problems();

        if problems.is_empty() {
            return Ok("No problems found.".to_owned());
        }

        Ok(problems
            .iter()
            .map(Problem::text)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

impl Check {
    /// Loads `source` and all imported files.
    fn new(source: Source) -> Result<Self, DiagnosticError> {
        let (entry, memory) = source.documents();
        let project = project_of(&entry, None)?;
        let provider = FsProvider::new(PathResolution::new(project.manifest.extensions.clone()))
            .with_memory(Arc::new(memory));
        let mut options = Analyzer::document_options(Dialect::GreyScript);

        options.parser.strict_mode = project.manifest.strict_mode;
        options.parser.syntax_features = project.manifest.syntax.features();

        let mut workspace = Workspace::with_database(Database::default(), options);

        workspace.load(&entry, &provider)?;

        let mut files: Vec<Uri> = workspace
            .files()
            .map(|file| file.uri(workspace.db()).clone())
            .filter(|uri| *uri != entry)
            .collect();

        files.sort();
        files.insert(0, entry);

        Ok(Self {
            workspace,
            files,
            options: project.manifest.diagnostic_options(),
        })
    }

    /// Per-file diagnostics in file order, sorted by source position.
    fn problems(&self) -> Vec<Problem> {
        let analyzer = Analyzer::new(&self.workspace);

        self.files
            .iter()
            .flat_map(|uri| {
                let mut diagnostics = self.options.apply(analyzer.all_diagnostics(uri));

                diagnostics.sort_by_key(|diagnostic| {
                    (
                        diagnostic.range.start.line,
                        diagnostic.range.start.character,
                    )
                });

                diagnostics.into_iter().map(|diagnostic| Problem {
                    uri: uri.clone(),
                    diagnostic,
                })
            })
            .collect()
    }
}

impl Problem {
    /// Formats a diagnostic as `file:line:character: severity: message`.
    fn text(&self) -> String {
        let start = self.diagnostic.range.start;
        let severity = match self.diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Information => "information",
            Severity::Hint => "hint",
        };

        format!(
            "{}: {severity}: {}",
            place(&self.uri.to_string(), start.line, start.character),
            self.diagnostic.message
        )
    }
}
