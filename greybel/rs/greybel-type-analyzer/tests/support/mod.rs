//! Shared analysis, source-marker, and workspace helpers for integration tests.

#![allow(dead_code)]

pub mod assignments;
pub mod call_hierarchy;
pub mod code_actions;
pub mod custom_types;
pub mod diagnostics;
pub mod doc;
pub mod documents;
pub mod features;
pub mod ignores;
pub mod import_path;
pub mod inlay;
pub mod keywords;
pub mod ranges;
pub mod type_definition;
pub mod infer;
pub mod instances;
pub mod narrowing;
pub mod type_expr;
pub mod workspace;

use greybel_core::Position;
use greybel_type_analyzer::{Analysis, Analyzer, Database, Dialect, ScopeId, Uri, Workspace};

pub const PATH: &str = "/main.src";

pub fn uri(path: &str) -> Uri {
    Uri::memory(path)
}

/// Creates an empty workspace using the analyzer's document options for `dialect`.
pub fn new_workspace(dialect: Dialect) -> Workspace<Database> {
    Workspace::with_database(Database::default(), Analyzer::document_options(dialect))
}

/// Creates a workspace containing only `source` at [`PATH`].
pub fn with_document(dialect: Dialect, source: &str) -> Workspace<Database> {
    let mut workspace = new_workspace(dialect);

    workspace.set_file(&uri(PATH), source);

    workspace
}

pub fn globals(dialect: Dialect, source: &str, names: &[&str]) -> Vec<String> {
    globals_in(&with_document(dialect, source), PATH, names)
}

/// Returns displayed types of the globals `names` in the file at `path`.
pub fn globals_in(workspace: &Workspace<Database>, path: &str, names: &[&str]) -> Vec<String> {
    let analysis = Analyzer::new(workspace)
        .analysis(&uri(path))
        .unwrap_or_else(|| panic!("{path} is loaded"));

    names.iter().map(|name| global(analysis, name)).collect()
}

/// Returns the displayed type of global `name` in the document at [`PATH`].
pub fn main_global(workspace: &Workspace<Database>, name: &str) -> String {
    globals_in(workspace, PATH, &[name]).remove(0)
}

pub fn global(analysis: &Analysis, name: &str) -> String {
    let ty = analysis
        .global_type(name)
        .unwrap_or_else(|| panic!("no global `{name}`"));

    analysis.display(ty).to_string()
}

/// Returns displayed types of `names` in the scope of the source's `index`th function.
pub fn locals(analysis: &Analysis, index: usize, names: &[&str]) -> Vec<String> {
    let function = analysis.model().functions().nth(index).expect("a function");
    let scope = ScopeId::Function(function);

    names
        .iter()
        .map(|name| {
            let ty = analysis
                .variable_type(scope, name)
                .unwrap_or_else(|| panic!("no local `{name}`"));

            analysis.display(ty).to_string()
        })
        .collect()
}

pub fn miniscript(source: &str, names: &[&str]) -> Vec<String> {
    globals(Dialect::MiniScript, source, names)
}

pub fn greyscript(source: &str, names: &[&str]) -> Vec<String> {
    globals(Dialect::GreyScript, source, names)
}

/// Removes every `|` marker from `source` and returns the resulting text and each marker's 1-based position.
pub fn marked(source: &str) -> (String, Vec<Position>) {
    let mut text = String::new();
    let mut positions = Vec::new();
    let (mut line, mut column) = (1, 1);

    for c in source.chars() {
        if c == '|' {
            positions.push(Position::new(line, column));
            continue;
        }

        text.push(c);

        if c == '\n' {
            line += 1;
            column = 1;
        } else {
            column += u32::try_from(c.len_utf16()).expect("value fits the target integer type");
        }
    }

    (text, positions)
}
