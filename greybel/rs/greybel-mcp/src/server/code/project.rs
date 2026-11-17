use std::path::Path;

use greybel_core::{ColumnSettings, LineIndex, Position};
use greybel_documents::{PathResolution, ResourceProvider, Uri, Workspace};
use greybel_documents_fs_provider::FsProvider;
use greybel_type_analyzer::{Analyzer, Database, Dialect};

use super::CodeError;
use crate::server::source::{current_folder, place, project_of};

/// Every script of a project, loaded and analysed as the language server does: the project is the folder of the
/// nearest `greybel.json` at or above `path`; else the current folder when `path` is in it or absent, else the
/// folder of `path`.
pub struct CodeProject {
    workspace: Workspace<Database>,
    files: Vec<Uri>,
}

impl CodeProject {
    pub fn open(path: Option<&str>) -> Result<Self, CodeError> {
        let current = current_folder();
        let folder = match path.map(Path::new) {
            Some(path) if path.is_dir() => FsProvider::uri(path),
            Some(path) if path.is_file() => FsProvider::uri(path)
                .dirname()
                .unwrap_or_else(|_| current.clone()),
            Some(path) => return Err(CodeError::UnknownFile(path.display().to_string())),
            None => current.clone(),
        };
        let manifest_location = folder
            .join_path(&[greybel_project::Project::FILE_NAME])
            .unwrap_or_else(|_| folder.clone());
        let project = project_of(&manifest_location, None)?;
        let is_in_current = folder.to_string().starts_with(&current.to_string());
        let root = project
            .root
            .clone()
            .unwrap_or(if is_in_current { current } else { folder });
        let manifest = &project.manifest;
        let provider =
            FsProvider::new(PathResolution::new(manifest.extensions.clone()).with_folders([root]));
        let pattern = match manifest.extensions.as_slice() {
            [extension] => format!("**/*.{extension}"),
            extensions => format!("**/*.{{{}}}", extensions.join(",")),
        };
        let exclude = (!manifest.exclude.is_empty()).then_some(manifest.exclude.as_str());
        let files = provider.find(&pattern, exclude);
        let mut options = Analyzer::document_options(Dialect::GreyScript);

        options.parser.strict_mode = manifest.strict_mode;
        options.parser.syntax_features = manifest.syntax.features();

        let mut workspace = Workspace::with_database(Database::default(), options);

        for file in &files {
            if provider.read(file).ok().flatten().is_some() {
                workspace.load(file, &provider)?;
            }
        }

        Ok(Self { workspace, files })
    }

    pub fn analyzer(&self) -> Analyzer<'_> {
        Analyzer::new(&self.workspace)
    }

    pub fn files(&self) -> &[Uri] {
        &self.files
    }

    /// The project file at `path`.
    pub fn file(&self, path: &str) -> Result<Uri, CodeError> {
        let uri = FsProvider::uri(Path::new(path));

        if self.workspace.file(&uri).is_some() {
            Ok(uri)
        } else {
            Err(CodeError::UnknownFile(path.to_owned()))
        }
    }

    pub fn text(&self, uri: &Uri) -> Option<String> {
        self.workspace.text(uri).map(|text| text.to_string())
    }

    /// The text of line `line` (one-based), trimmed.
    pub fn line_text(&self, uri: &Uri, line: u32) -> String {
        self.text(uri)
            .and_then(|text| {
                text.lines()
                    .nth(line.saturating_sub(1) as usize)
                    .map(|line| line.trim().to_owned())
            })
            .unwrap_or_default()
    }

    /// The text between `start` and `end`.
    pub fn slice(&self, uri: &Uri, start: Position, end: Position) -> Option<String> {
        let text = self.text(uri)?;
        let lines = LineIndex::new(&text);
        let start = lines.offset(start, ColumnSettings::default())? as usize;
        let end = lines.offset(end, ColumnSettings::default())? as usize;

        text.get(start..end).map(str::to_owned)
    }

    /// `path:line:column` of `position` in `uri`.
    pub fn place(uri: &Uri, position: Position) -> String {
        place(&uri.to_string(), position.line, position.character)
    }
}
