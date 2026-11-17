use std::path::Path;

use greybel_core::Position;
use greybel_documents::Uri;
use schemars::JsonSchema;
use serde::Deserialize;

use super::index::SymbolIndex;
use super::{CodeError, CodeProject};

/// A symbol given by name, or a position in a file.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SymbolTarget {
    /// Name or name path of a symbol, such as `format` or `App.utils.format`. Members are joined to their owner
    /// with dots; a function's locals follow its name (`main.count`).
    pub symbol: Option<String>,
    /// File of the symbol or position, or a folder of the project. The project is the nearest `greybel.json` at
    /// or above it, else its folder; the current folder when absent.
    pub path: Option<String>,
    /// One-based line of the position in `path`.
    pub line: Option<u32>,
    /// One-based column of the position in `path`.
    pub column: Option<u32>,
}

impl SymbolTarget {
    pub fn open_project(&self) -> Result<CodeProject, CodeError> {
        CodeProject::open(self.path.as_deref())
    }

    /// The file and position the target names.
    pub fn locate(&self, project: &CodeProject) -> Result<(Uri, Position), CodeError> {
        let file = match self.path.as_deref() {
            Some(path) if Path::new(path).is_file() => Some(project.file(path)?),
            _ => None,
        };

        match (&self.symbol, file, self.line, self.column) {
            (Some(name), file, None, None) => {
                let index = SymbolIndex::new(project);
                let symbol = index.resolve(name, file.as_ref())?;

                Ok((symbol.uri.clone(), symbol.name_position()))
            }
            (None, Some(file), Some(line), Some(column)) => Ok((file, Position::new(line, column))),
            _ => Err(CodeError::Target),
        }
    }
}
