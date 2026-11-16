//! Tool source loading, project discovery, and source-location formatting.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use greybel_builder::protocol::BuildFailure;
use greybel_documents::Uri;
use greybel_documents_fs_provider::FsProvider;
use greybel_project::{Project, ProjectError as ManifestError};

/// Virtual current-folder document used for request-supplied code and its imports.
const CODE_FILE: &str = "code.src";

/// Converts a tool error to rmcp text content.
macro_rules! into_tool_contents {
    ($error:ty) => {
        impl rmcp::model::IntoContents for $error {
            fn into_contents(self) -> Vec<rmcp::model::ContentBlock> {
                rmcp::model::IntoContents::into_contents(self.to_string())
            }
        }
    };
}

pub(crate) use into_tool_contents;

/// Reason a requested source cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("Provide exactly one of code or path.")]
    Selection,
    #[error("There is no file at {}.", path.display())]
    MissingFile { path: PathBuf },
}

/// Request-supplied code or an on-disk file.
pub enum Source {
    Code(String),
    File(Uri),
}

impl Source {
    /// Source selected by exactly one request field: `code` or `path`.
    pub fn of(code: Option<String>, path: Option<String>) -> Result<Self, SourceError> {
        match (code, path) {
            (Some(code), None) => Ok(Self::Code(code)),
            (None, Some(path)) => existing_file(&path).map(Self::File),
            _ => Err(SourceError::Selection),
        }
    }

    /// Entry document and in-memory filesystem overrides: `code.src` for supplied code, or no overrides.
    pub fn documents(self) -> (Uri, BTreeMap<Uri, String>) {
        match self {
            Self::Code(code) => {
                let entry = FsProvider::uri(Path::new(CODE_FILE));

                (entry.clone(), BTreeMap::from([(entry, code)]))
            }
            Self::File(entry) => (entry, BTreeMap::new()),
        }
    }
}

/// Finds the project for `uri` from the nearest `greybel.json` at or above its folder, applies `profile`, or uses
/// defaults. Request-supplied code uses the current folder's project.
pub fn project_of(uri: &Uri, profile: Option<&str>) -> Result<Project, ManifestError> {
    Project::discover(uri, FsProvider::text).and_then(|project| project.with_profile(profile))
}

/// Base folder for files without `greybel.json`: the current folder.
pub fn current_folder() -> Uri {
    FsProvider::uri(Path::new("."))
}

/// Resolves `path` to a file URI or returns the resolution error.
pub fn existing_file(path: &str) -> Result<Uri, SourceError> {
    let path = Path::new(path);

    if path.is_file() {
        Ok(FsProvider::uri(path))
    } else {
        Err(SourceError::MissingFile {
            path: path.to_owned(),
        })
    }
}

/// Formats a location as `path:line:character` using `FsProvider::display_path`.
pub fn place(uri: &str, line: u32, character: u32) -> String {
    let path =
        Uri::parse(uri).map_or_else(|_| uri.to_owned(), |uri| FsProvider::display_path(&uri));

    format!("{path}:{line}:{character}")
}

/// Build, import, or transformation failure with an optional source location.
pub fn failure_text(failure: &BuildFailure) -> String {
    match failure {
        BuildFailure::Invalid {
            location: Some(location),
            ..
        } => format!(
            "{failure} at {}",
            place(&location.uri, location.line, location.character)
        ),
        _ => failure.to_string(),
    }
}
