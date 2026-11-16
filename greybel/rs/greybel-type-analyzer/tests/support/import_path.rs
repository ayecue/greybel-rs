use greybel_core::Position;
use greybel_documents_memory_provider::MemoryProvider;
use greybel_type_analyzer::{Analyzer, Database, Dialect, Workspace};

use super::uri;
use super::workspace::load;

pub const ENTRY: &str = "/app/main.src";

/// A library with a comment at its start, a function and a variable assigned twice.
pub const MATH: (&str, &str) = (
    "/app/lib/math.src",
    "// Math helpers.\n\nadd = function(a, b)\n  return a + b\nend function\nPI = 3.14\nPI = 3\n",
);

fn workspace(dialect: Dialect, entry: &str, files: &[(&str, &str)]) -> Workspace<Database> {
    let provider =
        MemoryProvider::from_iter(std::iter::once((ENTRY, entry)).chain(files.iter().copied()));

    load(dialect, &provider, &[ENTRY])
}

/// The hover text and description at `position` of [`ENTRY`], with `files` in the workspace.
pub fn hover_at(
    dialect: Dialect,
    entry: &str,
    files: &[(&str, &str)],
    position: Position,
) -> Option<(String, String)> {
    let workspace = workspace(dialect, entry, files);
    let hover = Analyzer::new(&workspace).hover(&uri(ENTRY), position)?;
    let description = hover
        .documentation
        .and_then(|documentation| documentation.description)
        .unwrap_or_default();

    Some((hover.text, description))
}

/// The path and start of each definition at `position` of [`ENTRY`].
pub fn definition_at(
    dialect: Dialect,
    entry: &str,
    files: &[(&str, &str)],
    position: Position,
) -> Vec<(String, Position)> {
    let workspace = workspace(dialect, entry, files);

    Analyzer::new(&workspace)
        .definition(&uri(ENTRY), position)
        .into_iter()
        .map(|location| (location.uri.path().to_owned(), location.range.start))
        .collect()
}

/// The start and end of the range a definition at `position` of [`ENTRY`] starts from.
pub fn origin_at(
    dialect: Dialect,
    entry: &str,
    files: &[(&str, &str)],
    position: Position,
) -> Option<(Position, Position)> {
    let workspace = workspace(dialect, entry, files);

    Analyzer::new(&workspace)
        .definition_origin(&uri(ENTRY), position)
        .map(|range| (range.start, range.end))
}
